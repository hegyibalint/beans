use super::*;
use beans_testing::template::Template;
use lsp_server::Notification;
use serde_json::json;

#[test]
fn document_notifications_and_hover_requests_reach_features() {
    let mut server = server(Lifecycle::Running);
    let uri = "untitled:Example.java";
    let initial = Template::parse("class C extends <cur>Box {}");
    let changed = Template::parse("class C extends <cur>NewType {}");
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": uri, "languageId": "java", "version": 1,
            "text": initial.content
        }}),
    ));
    let hover_at = |server: &mut Server, fixture: &Template| {
        request(
            server,
            "textDocument/hover",
            json!({"textDocument": {"uri": uri},
                   "position": position(fixture, fixture.cursors[0].offset)}),
        )
        .result
        .unwrap()
    };
    assert_eq!(hover_at(&mut server, &initial)["contents"]["value"], "Box");

    server.notify(Notification::new(
        "textDocument/didChange".into(),
        json!({"textDocument": {"uri": uri, "version": 2},
               "contentChanges": [{"text": changed.content}]}),
    ));
    assert_eq!(
        hover_at(&mut server, &changed)["contents"]["value"],
        "NewType"
    );

    server.notify(Notification::new(
        "textDocument/didClose".into(),
        json!({"textDocument": {"uri": uri}}),
    ));
    assert_eq!(hover_at(&mut server, &changed), Value::Null);
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
