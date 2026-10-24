use lsp_server::Notification;
use serde_json::json;

use super::{request, server};

#[test]
fn full_changes_replace_the_document_and_rejected_changes_leave_it_current() {
    let (_root, mut client) = server();
    let uri = "untitled:Example.java";
    client.notify(Notification::new("textDocument/didOpen".into(), json!({
        "textDocument": {"uri": uri, "languageId": "java", "version": 1, "text": "class C extends Old {}"}
    })));
    let publications = client.notify(Notification::new(
        "textDocument/didChange".into(),
        json!({
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": "class C extends New {}"}]
        }),
    ));
    assert_eq!(publications.len(), 1);
    assert_eq!(publications[0].params["version"], 2);

    for params in [
        json!({"textDocument": {"uri": uri, "version": 1}, "contentChanges": [{"text": "class C extends Stale {}"}]}),
        json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": "class C extends Repeated {}"}]}),
        json!({"textDocument": {"uri": uri, "version": 3}, "contentChanges": []}),
        json!({"textDocument": {"uri": uri, "version": 3}, "contentChanges": [{"text": "class C extends One {}"}, {"text": "class C extends Two {}"}]}),
        json!({"textDocument": {"uri": uri, "version": 3}, "contentChanges": [{"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}}, "text": "x"}]}),
    ] {
        assert!(
            client
                .notify(Notification::new("textDocument/didChange".into(), params))
                .is_empty()
        );
        let hover = request(
            &mut client,
            "textDocument/hover",
            json!({
                "textDocument": {"uri": uri}, "position": {"line": 0, "character": 16}
            }),
        );
        assert_eq!(hover.result.unwrap()["contents"]["value"], "New");
    }

    let publications = client.notify(Notification::new("textDocument/didChange".into(), json!({
        "textDocument": {"uri": uri, "version": 3}, "contentChanges": [{"text": "class C extends Final {}"}]
    })));
    assert_eq!(publications.len(), 1);
    assert_eq!(publications[0].params["version"], 3);
    let hover = request(
        &mut client,
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri}, "position": {"line": 0, "character": 16}
        }),
    );
    assert_eq!(hover.result.unwrap()["contents"]["value"], "Final");
}
