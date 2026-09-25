use super::*;
use lsp_server::Notification;
use serde_json::json;

#[test]
fn document_notifications_and_hover_requests_reach_features() {
    let mut server = server(Lifecycle::Running);
    let uri = "untitled:Example.java";
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": uri, "languageId": "java", "version": 1,
            "text": "class C extends Box {}"
        }}),
    ));
    let hover_at = |server: &mut Server| {
        request(
            server,
            "textDocument/hover",
            json!({"textDocument": {"uri": uri}, "position": {"line": 0, "character": 16}}),
        )
        .result
        .unwrap()
    };
    assert_eq!(hover_at(&mut server)["contents"]["value"], "Box");

    server.notify(Notification::new(
        "textDocument/didChange".into(),
        json!({"textDocument": {"uri": uri, "version": 2},
               "contentChanges": [{"text": "class C extends NewType {}"}]}),
    ));
    assert_eq!(hover_at(&mut server)["contents"]["value"], "NewType");

    server.notify(Notification::new(
        "textDocument/didClose".into(),
        json!({"textDocument": {"uri": uri}}),
    ));
    assert_eq!(hover_at(&mut server), Value::Null);
}

#[test]
fn invalid_hover_parameters_are_rejected() {
    assert_error(
        request(
            &mut server(Lifecycle::Running),
            "textDocument/hover",
            Value::Null,
        ),
        ErrorCode::InvalidParams,
    );
}
