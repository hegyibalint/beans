use super::*;
use beans_testing::template::Template;
use lsp_server::Notification;
use serde_json::json;

#[test]
fn hover_requests_reach_features_and_closed_documents_return_null() {
    let mut server = server(Lifecycle::Running);
    let uri = "untitled:Example.java";
    let fixture = Template::parse("class C extends <cur>Box {}");
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": uri, "languageId": "java", "version": 1,
            "text": fixture.content
        }}),
    ));
    let hover_at = |server: &mut Server| {
        request(
            server,
            "textDocument/hover",
            json!({"textDocument": {"uri": uri},
                   "position": position(&fixture, fixture.cursors[0].offset)}),
        )
        .result
        .unwrap()
    };
    assert_eq!(hover_at(&mut server)["contents"]["kind"], "plaintext");

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
