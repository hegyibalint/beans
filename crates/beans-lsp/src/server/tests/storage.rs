use beans_core::revision::Revision;
use lsp_server::Notification;
use serde_json::{Value, json};

use super::{Lifecycle, assert_error, request, server};
use crate::server::Server;

fn open(server: &mut Server, text: &str) -> Vec<Notification> {
    server.notify(Notification::new("textDocument/didOpen".into(), json!({
        "textDocument": {"uri": "untitled:Example.java", "languageId": "java", "version": 1, "text": text}
    })))
}

fn hover(server: &mut Server) -> lsp_server::Response {
    request(
        server,
        "textDocument/hover",
        json!({
            "textDocument": {"uri": "untitled:Example.java"},
            "position": {"line": 0, "character": 16}
        }),
    )
}

fn assert_reported(notifications: &[Notification], message: &str) {
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].method, "window/showMessage");
    assert_eq!(notifications[0].params["type"], 1);
    assert!(
        notifications[0].params["message"]
            .as_str()
            .unwrap()
            .contains(message)
    );
}

#[test]
fn failed_open_reports_an_error_and_the_server_can_recover() {
    let (root, mut server) = server();
    let connection = beans_engine::storage::open(&root.path().join(".beans/models")).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_type BEFORE INSERT ON java_types BEGIN SELECT RAISE(ABORT, 'publication rejected'); END;").unwrap();
    assert_reported(
        &open(&mut server, "class C extends Target {}"),
        "publication rejected",
    );
    assert_eq!(
        server.features.as_ref().unwrap().revision(),
        Revision::default()
    );
    assert_eq!(hover(&mut server).result, Some(Value::Null));
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    connection.execute("DROP TRIGGER reject_type", []).unwrap();
    assert!(
        open(&mut server, "class C extends Target {}")
            .iter()
            .all(|message| message.method == "textDocument/publishDiagnostics")
    );
    assert_eq!(
        hover(&mut server).result.unwrap()["contents"]["value"],
        "Target"
    );
}

#[test]
fn failed_changes_keep_the_last_published_document_and_allow_retry() {
    let (root, mut server) = server();
    open(&mut server, "class C extends Former {}");
    let connection = beans_engine::storage::open(&root.path().join(".beans/models")).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_type BEFORE INSERT ON java_types BEGIN SELECT RAISE(ABORT, 'change rejected'); END;").unwrap();
    let change = || {
        Notification::new(
            "textDocument/didChange".into(),
            json!({
                "textDocument": {"uri": "untitled:Example.java", "version": 2},
                "contentChanges": [{"text": "class C extends Latest {}"}]
            }),
        )
    };
    assert_reported(&server.notify(change()), "change rejected");
    assert_eq!(
        server.features.as_ref().unwrap().revision(),
        Revision::new(1)
    );
    assert_eq!(
        hover(&mut server).result.unwrap()["contents"]["value"],
        "Former"
    );
    connection.execute("DROP TRIGGER reject_type", []).unwrap();
    server.notify(change());
    assert_eq!(
        server.features.as_ref().unwrap().revision(),
        Revision::new(2)
    );
    assert_eq!(
        hover(&mut server).result.unwrap()["contents"]["value"],
        "Latest"
    );
}

#[test]
fn failed_close_does_not_discard_the_open_document() {
    let (root, mut server) = server();
    open(&mut server, "class C extends Target {}");
    let connection = beans_engine::storage::open(&root.path().join(".beans/models")).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_removal BEFORE INSERT ON entries WHEN NEW.model IS NULL BEGIN SELECT RAISE(ABORT, 'close rejected'); END;").unwrap();
    let close = || {
        Notification::new(
            "textDocument/didClose".into(),
            json!({"textDocument": {"uri": "untitled:Example.java"}}),
        )
    };
    assert_reported(&server.notify(close()), "close rejected");
    assert_eq!(
        hover(&mut server).result.unwrap()["contents"]["value"],
        "Target"
    );
    connection
        .execute("DROP TRIGGER reject_removal", [])
        .unwrap();
    server.notify(close());
    assert_eq!(hover(&mut server).result, Some(Value::Null));
}

#[test]
fn decoding_errors_become_request_errors_without_stopping_the_server() {
    let (root, mut server) = server();
    open(&mut server, "class C extends Target {}");
    let connection = beans_engine::storage::open(&root.path().join(".beans/models")).unwrap();
    connection
        .execute("UPDATE entries SET model = '{}'", [])
        .unwrap();
    for method in [
        "textDocument/hover",
        "textDocument/definition",
        "textDocument/semanticTokens/full",
    ] {
        let response = request(
            &mut server,
            method,
            json!({
                "textDocument": {"uri": "untitled:Example.java"},
                "position": {"line": 0, "character": 16}
            }),
        );
        assert_error(response, lsp_server::ErrorCode::RequestFailed);
        assert!(matches!(server.lifecycle, Lifecycle::Running));
    }
    open(&mut server, "class C extends Recovered {}");
    assert_eq!(
        hover(&mut server).result.unwrap()["contents"]["value"],
        "Recovered"
    );
}
