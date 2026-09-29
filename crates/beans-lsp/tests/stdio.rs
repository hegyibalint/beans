use std::{
    fs,
    io::BufReader,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use lsp_server::{Message, Notification, Request};
use serde_json::json;

#[test]
fn stdio_serves_a_session_and_exits_without_waiting_for_eof() {
    let log_path = std::env::temp_dir().join(format!(
        "beans-lsp-{}-{}.jsonl",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut child = Command::new(env!("CARGO_BIN_EXE_beans-lsp"))
        .env("BEANS_LSP_LOG_PATH", &log_path)
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
            Message::Request(Request::new(4.into(), "unknown/method".into(), ()))
                .write(&mut stdin)?;
            let Some(Message::Response(response)) = Message::read(&mut stdout)? else {
                panic!("expected error response");
            };
            assert_eq!(response.id, 4.into());
            assert_eq!(response.error.unwrap().code, -32601);
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
    let lines = fs::read_to_string(&log_path).unwrap();
    fs::remove_file(log_path).unwrap();
    let entries: Vec<serde_json::Value> = lines
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(entries.iter().any(|entry| entry["direction"] == "incoming"
        && entry["message"]["method"] == "textDocument/didOpen"
        && entry["message"]["params"]["textDocument"]["text"] == "class Example {}"));
    assert!(entries.iter().any(|entry| entry["direction"] == "outgoing"
        && entry["message"]["id"] == 1
        && entry["message"]["result"]["serverInfo"]["name"] == "beans"));
    assert!(entries.iter().any(|entry| entry["direction"] == "incoming"
        && entry["message"]["method"] == "unknown/method"
        && entry["message"]["id"] == 4));
    assert!(entries.iter().any(|entry| entry["direction"] == "outgoing"
        && entry["message"]["id"] == 4
        && entry["message"]["error"]["code"] == -32601));
}
