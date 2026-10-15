use std::{
    fs,
    io::{BufReader, Read, Write},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use lsp_server::{Message, Notification, Request};
use serde_json::{Value, json};
use tempfile::tempdir;
use url::Url;

#[test]
fn stdio_serves_sqlite_hover_and_definition_and_exits_without_waiting_for_eof() {
    let root = tempdir().unwrap();
    fs::create_dir_all(root.path().join("src/q")).unwrap();
    fs::write(
        root.path().join("beans.toml"),
        "[module.app]\nsources = ['src']\n",
    )
    .unwrap();
    let target_text = "package q; /*😀*/ public class Target {}";
    let target = root.path().join("src/q/Target.java");
    fs::write(&target, target_text).unwrap();
    let target_uri = Url::from_file_path(&target).unwrap().to_string();
    let target_start = target_text[..target_text.find("Target").unwrap()]
        .encode_utf16()
        .count();
    let log_path = root.path().join("traffic.jsonl");
    let mut child = Command::new(env!("CARGO_BIN_EXE_beans-lsp"))
        .env("BEANS_LSP_LOG_PATH", &log_path)
        .env("RUST_LOG", "beans=debug")
        .env("BEANS_WORKSPACE_TOML_JAVA_HOME", "missing-jdk")
        .env_remove("BEANS_LSP_DATABASE_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let workspace_uri = Url::from_directory_path(root.path()).unwrap().to_string();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        let result = (|| -> std::io::Result<()> {
            let initialized = exchange(
                &mut stdin,
                &mut stdout,
                1,
                "initialize",
                json!({
                    "capabilities": {"textDocument": {"definition": {"linkSupport": true}}},
                    "workspaceFolders": [{"uri": workspace_uri, "name": "test"}]
                }),
            )?;
            assert_eq!(initialized["capabilities"]["hoverProvider"], true);
            assert_eq!(initialized["capabilities"]["definitionProvider"], true);
            notify(&mut stdin, "initialized", json!({}))?;
            let uri = "untitled:Untitled-1";
            let text = "package p; import q.Target; class Café /*😀*/ { Target field; }";
            let start = text[..text.rfind("Target").unwrap()].encode_utf16().count();
            notify(
                &mut stdin,
                "textDocument/didOpen",
                json!({"textDocument": {
                    "uri": uri, "languageId": "java", "version": 1, "text": text
                }}),
            )?;
            let params =
                json!({"textDocument": {"uri": uri}, "position": {"line": 0, "character": start}});
            let hover = exchange(
                &mut stdin,
                &mut stdout,
                2,
                "textDocument/hover",
                params.clone(),
            )?;
            assert_eq!(
                hover,
                json!({"contents": {"kind": "plaintext", "value": "Target"},
                "range": {"start": {"line": 0, "character": start}, "end": {"line": 0, "character": start + 6}}})
            );
            let definition = exchange(
                &mut stdin,
                &mut stdout,
                3,
                "textDocument/definition",
                params.clone(),
            )?;
            assert_eq!(definition[0]["targetUri"], target_uri);
            assert_eq!(
                definition[0]["targetSelectionRange"],
                json!({"start": {"line": 0, "character": target_start}, "end": {"line": 0, "character": target_start + 6}})
            );
            assert_eq!(definition[0]["originSelectionRange"], hover["range"]);
            notify(
                &mut stdin,
                "textDocument/didChange",
                json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": text.replace("Target field", "Other field")}]}),
            )?;
            let edited = exchange(
                &mut stdin,
                &mut stdout,
                4,
                "textDocument/hover",
                params.clone(),
            )?;
            assert_eq!(edited["contents"]["value"], "Other");
            assert!(
                exchange(
                    &mut stdin,
                    &mut stdout,
                    5,
                    "textDocument/definition",
                    params.clone()
                )?
                .is_null()
            );
            notify(
                &mut stdin,
                "textDocument/didChange",
                json!({"textDocument": {"uri": uri, "version": 1}, "contentChanges": [{"text": text}]}),
            )?;
            assert_eq!(
                exchange(
                    &mut stdin,
                    &mut stdout,
                    6,
                    "textDocument/hover",
                    params.clone()
                )?["contents"]["value"],
                "Other"
            );
            notify(
                &mut stdin,
                "textDocument/didClose",
                json!({"textDocument": {"uri": uri}}),
            )?;
            assert!(exchange(&mut stdin, &mut stdout, 7, "textDocument/hover", params)?.is_null());
            assert!(exchange(&mut stdin, &mut stdout, 8, "shutdown", Value::Null)?.is_null());
            notify(&mut stdin, "exit", Value::Null)?;
            // Keep stdin open: exit must terminate the process without waiting for EOF.
            assert!(Message::read(&mut stdout)?.is_none());
            Ok(())
        })();
        let _ = sender.send(result);
    });
    let result = receiver.recv_timeout(Duration::from_secs(20));
    if !matches!(result, Ok(Ok(()))) {
        let _ = child.kill();
        let _ = child.wait();
        let mut diagnostics = String::new();
        stderr.read_to_string(&mut diagnostics).unwrap();
        panic!("stdio session failed: {result:?}\n{diagnostics}");
    }
    assert!(child.wait().unwrap().success());
    assert!(root.path().join(".beans/index.sqlite").is_file());
    let mut diagnostics = String::new();
    stderr.read_to_string(&mut diagnostics).unwrap();
    assert!(diagnostics.contains("Loaded "), "{diagnostics}");
    assert!(
        diagnostics.contains("no supported JDK runtime"),
        "{diagnostics}"
    );
    let traffic = fs::read_to_string(log_path).unwrap();
    assert!(traffic.contains("textDocument/hover"));
    assert!(traffic.contains("textDocument/definition"));
    assert!(traffic.contains("Target"));
    let engine = beans_engine::Engine::new(&root.path().join(".beans/index.sqlite")).unwrap();
    let resource =
        beans_core::resource::ResourceId::new(beans_core::resource::ResourceRoot::Virtual {
            provider: "lsp".into(),
            key: "untitled:Untitled-1".into(),
        });
    let text = "package p; import q.Target; class Café /*😀*/ { Other field; }";
    assert_eq!(
        engine
            .feature_hover(&beans_core::features::HoverRequest {
                source: &resource,
                contents: text,
                offset: text.rfind("Other").unwrap()
            })
            .unwrap()
            .unwrap()
            .contents(),
        "Other"
    );
}

#[test]
fn database_creation_failure_is_fatal_to_the_stdio_server() {
    let root = tempdir().unwrap();
    fs::write(root.path().join(".beans"), "not a directory").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_beans-lsp"))
        .env_remove("BEANS_LSP_DATABASE_PATH")
        .env_remove("BEANS_LSP_LOG_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    Message::Request(Request::new(1.into(), "initialize".into(), json!({"capabilities": {},
        "workspaceFolders": [{"uri": Url::from_directory_path(root.path()).unwrap().as_str(), "name": "test"}]
    }))).write(&mut stdin).unwrap();
    let (done, complete) = mpsc::channel();
    thread::spawn(move || {
        let _ = done.send(child.wait_with_output().unwrap());
    });
    let output = complete.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot start Engine storage"));
}

fn exchange(
    stdin: &mut impl Write,
    stdout: &mut impl std::io::BufRead,
    id: i32,
    method: &str,
    params: Value,
) -> std::io::Result<Value> {
    Message::Request(Request::new(id.into(), method.into(), params)).write(stdin)?;
    let Some(Message::Response(response)) = Message::read(stdout)? else {
        panic!("expected response for {method}");
    };
    assert_eq!(response.id, id.into());
    assert!(response.error.is_none(), "{method}: {:?}", response.error);
    Ok(response.result.unwrap_or(Value::Null))
}

fn notify(stdin: &mut impl Write, method: &str, params: Value) -> std::io::Result<()> {
    Message::Notification(Notification::new(method.into(), params)).write(stdin)
}
