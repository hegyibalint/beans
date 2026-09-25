use super::*;
use lsp_server::Notification;
use serde_json::json;

#[test]
fn field_type_navigates_to_its_declaration_with_utf16_positions() {
    let mut server = server(Lifecycle::Running);
    let uri = "untitled:Example.java";
    let text = "class Café /*😀*/ { class Target {} Target field; }";
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": uri, "languageId": "java", "version": 1, "text": text
        }}),
    ));
    let character = |offset: usize| text[..offset].encode_utf16().count() as u32;
    let target = character(text.find("Target").unwrap());
    let use_position = character(text.rfind("Target").unwrap());

    let response = request(
        &mut server,
        "textDocument/definition",
        json!({"textDocument": {"uri": uri},
               "position": {"line": 0, "character": use_position}}),
    );

    assert_eq!(
        response.result,
        Some(json!({
            "uri": uri,
            "range": {
                "start": {"line": 0, "character": target},
                "end": {"line": 0, "character": target + "Target".len() as u32}
            }
        }))
    );
}

#[test]
fn goto_definition_without_an_open_document_returns_null() {
    let response = request(
        &mut server(Lifecycle::Running),
        "textDocument/definition",
        json!({"textDocument": {"uri": "untitled:Example.java"},
               "position": {"line": 0, "character": 0}}),
    );

    assert_eq!(response.result, Some(Value::Null));
}

#[test]
fn invalid_goto_definition_parameters_are_rejected() {
    assert_error(
        request(
            &mut server(Lifecycle::Running),
            "textDocument/definition",
            Value::Null,
        ),
        ErrorCode::InvalidParams,
    );
}
