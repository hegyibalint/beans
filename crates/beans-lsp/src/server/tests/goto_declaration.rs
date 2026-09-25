use super::*;
use serde_json::json;

#[test]
fn goto_declaration_without_an_open_document_returns_null() {
    let response = request(
        &mut server(Lifecycle::Running),
        "textDocument/declaration",
        json!({"textDocument": {"uri": "untitled:Example.java"},
               "position": {"line": 0, "character": 0}}),
    );

    assert_eq!(response.result, Some(Value::Null));
}

#[test]
fn invalid_goto_declaration_parameters_are_rejected() {
    assert_error(
        request(
            &mut server(Lifecycle::Running),
            "textDocument/declaration",
            Value::Null,
        ),
        ErrorCode::InvalidParams,
    );
}
