use super::*;
use beans_testing::{TempDir, template::Template};
use lsp_server::Notification;
use serde_json::json;

#[test]
fn hover_requests_reach_features_and_closed_documents_return_null() {
    let (_root, mut server) = server();
    let uri = "untitled:Example.java";
    let fixture = Template::parse("class C extends <cur>Box {}");
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": uri, "languageId": "java", "version": 1,
            "text": fixture.content
        }}),
    ));
    let hover_at = |server: &mut Client| {
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
    let (_root, mut server) = server();
    assert_error(
        request(&mut server, "textDocument/hover", Value::Null),
        ErrorCode::InvalidParams,
    );
}

#[test]
fn hover_respects_the_clients_markup_preference_order() {
    for (formats, expected) in [
        (
            json!(["markdown", "plaintext"]),
            json!({"kind": "markdown", "value": "**Type:** ` C `\n\n**Located in:** ` untitled:Example.java `"}),
        ),
        (
            json!(["plaintext", "markdown"]),
            json!({"kind": "plaintext", "value": "Type: C\nLocated in: untitled:Example.java"}),
        ),
    ] {
        let root = TempDir::new();
        let mut params = initialize_params(root.root());
        params["capabilities"] = json!({"textDocument": {"hover": {"contentFormat": formats}}});
        let mut client = Client::default();
        assert!(request(&mut client, "initialize", params).error.is_none());
        client.notify(Notification::new("textDocument/didOpen".into(), json!({
            "textDocument": {"uri": "untitled:Example.java", "languageId": "java", "version": 1, "text": "class C { C value; }"}
        })));
        let response = request(
            &mut client,
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "untitled:Example.java"}, "position": {"line": 0, "character": 10}
            }),
        );
        assert_eq!(response.result.unwrap()["contents"], expected);
    }
}

#[test]
fn hover_response_uses_plaintext_and_utf16_coordinates() {
    let (_root, mut client) = server();
    let text = "class Café /*😀*/ extends Box<Café> {}";
    client.notify(Notification::new("textDocument/didOpen".into(), json!({
        "textDocument": {"uri": "untitled:Example.java", "languageId": "java", "version": 1, "text": text}
    })));
    let start = text[..text.find("Box").unwrap()].encode_utf16().count();
    let end = start + "Box<Café>".encode_utf16().count();
    let response = request(
        &mut client,
        "textDocument/hover",
        json!({
            "textDocument": {"uri": "untitled:Example.java"}, "position": {"line": 0, "character": start}
        }),
    );
    assert_eq!(
        response.result,
        Some(json!({
            "contents": {"kind": "plaintext", "value": "Box<Café>"},
            "range": {"start": {"line": 0, "character": start}, "end": {"line": 0, "character": end}}
        }))
    );
}
