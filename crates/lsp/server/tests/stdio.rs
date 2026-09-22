use std::{
    io::BufReader,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use lsp_server::{Message, Notification, Request};
use serde_json::json;

#[test]
fn stdio_serves_a_session_and_exits_without_waiting_for_eof() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_beans-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        let result = (|| -> std::io::Result<()> {
            Message::Request(Request::new(
                1.into(),
                "initialize".into(),
                json!({"capabilities": {}}),
            ))
            .write(&mut stdin)?;
            let Some(Message::Response(response)) = Message::read(&mut stdout)? else {
                panic!("expected initialize response");
            };
            assert_eq!(response.id, 1.into());
            assert!(response.error.is_none());
            assert_eq!(response.result.unwrap()["serverInfo"]["name"], "beans");
            Message::Notification(Notification::new("initialized".into(), json!({})))
                .write(&mut stdin)?;
            Message::Notification(Notification::new(
                "textDocument/didOpen".into(),
                json!({
                    "textDocument": {
                        "uri": "file:///Example.java",
                        "languageId": "java",
                        "version": 1,
                        "text": "class Example {}"
                    }
                }),
            ))
            .write(&mut stdin)?;
            Message::Notification(Notification::new(
                "textDocument/didClose".into(),
                json!({"textDocument": {"uri": "file:///Example.java"}}),
            ))
            .write(&mut stdin)?;
            Message::Notification(Notification::new("$/unknown".into(), ())).write(&mut stdin)?;
            Message::Request(Request::new(2.into(), "shutdown".into(), ())).write(&mut stdin)?;
            let Some(Message::Response(response)) = Message::read(&mut stdout)? else {
                panic!("expected shutdown response");
            };
            assert_eq!(response.id, 2.into());
            assert!(response.error.is_none());
            // Serde deserializes a JSON null into None for Option<Value>.
            assert!(response.result.is_none());
            Message::Notification(Notification::new("exit".into(), ())).write(&mut stdin)?;
            // Keep stdin open: exit must terminate the process without waiting for EOF.
            assert!(Message::read(&mut stdout)?.is_none());
            Ok(())
        })();
        sender.send(result).unwrap();
    });
    let result = receiver.recv_timeout(Duration::from_secs(10));
    if !matches!(result, Ok(Ok(()))) {
        let _ = child.kill();
        let _ = child.wait();
        panic!("stdio session failed: {result:?}");
    }
    assert!(child.wait().unwrap().success());
}
