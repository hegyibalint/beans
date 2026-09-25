use super::*;
use lsp_server::Notification;
use serde_json::json;

#[test]
fn hovering_over_a_type_identifier_uses_the_open_document_and_utf16_range() {
    let mut server = server(Lifecycle::Running);
    let uri = "untitled:Example.java";
    let text = "class Café /*😀*/ extends Box<Café> {}";
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": uri, "languageId": "java", "version": 1, "text": text
        }}),
    ));
    let hover_at = |server: &mut Server, character: u32| {
        request(
            server,
            "textDocument/hover",
            json!({"textDocument": {"uri": uri}, "position": {"line": 0, "character": character}}),
        )
    };
    let start = text[..text.find("Box").unwrap()].encode_utf16().count() as u32;
    let response = hover_at(&mut server, start);
    assert_eq!(
        response.result.unwrap(),
        json!({
            "contents": {"kind": "plaintext", "value": "Box<Café>"},
            "range": {"start": {"line": 0, "character": start},
                      "end": {"line": 0, "character": start + 3}}
        })
    );
    assert_eq!(hover_at(&mut server, start + 3).result, Some(Value::Null));
    assert_eq!(hover_at(&mut server, 6).result, Some(Value::Null));

    server.notify(Notification::new(
        "textDocument/didChange".into(),
        json!({"textDocument": {"uri": uri, "version": 2},
               "contentChanges": [{"text": "class C extends NewType {}"}]}),
    ));
    assert_eq!(
        hover_at(&mut server, 16).result.unwrap()["contents"]["value"],
        "NewType"
    );
    server.notify(Notification::new(
        "textDocument/didClose".into(),
        json!({"textDocument": {"uri": uri}}),
    ));
    assert_eq!(hover_at(&mut server, 16).result, Some(Value::Null));
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
