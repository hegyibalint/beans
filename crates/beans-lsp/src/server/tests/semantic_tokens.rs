use lsp_server::Notification;
use lsp_types::SemanticTokens;
use serde_json::json;

use super::{initialize_params, request, server};
use crate::server::Server;

#[test]
fn initialization_advertises_full_type_tokens_only() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::default();
    let response = request(&mut server, "initialize", initialize_params(root.path()));
    let provider = &response.result.unwrap()["capabilities"]["semanticTokensProvider"];
    assert_eq!(
        provider["legend"],
        json!({"tokenTypes": ["type"], "tokenModifiers": []})
    );
    assert_eq!(provider["full"], true);
    assert!(provider.get("range").is_none());
}

#[test]
fn field_type_identifiers_are_sorted_and_utf16_encoded() {
    let (_root, mut server) = server();
    let uri = "untitled:Example.java";
    let text = "class Example { /* 😀 */ Box<Missing[]>[] a; Café b;\n pkg.Widget<? extends Absent> c; int[] counts; }";
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({
            "textDocument": {"uri": uri, "languageId": "java", "version": 1, "text": text}
        }),
    ));
    let response = request(
        &mut server,
        "textDocument/semanticTokens/full",
        json!({
            "textDocument": {"uri": uri}
        }),
    );
    let tokens: SemanticTokens = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(tokens.data.len(), 5);
    let mut line = 0;
    let mut character = 0;
    for (token, name) in tokens
        .data
        .iter()
        .zip(["Box", "Missing", "Café", "Widget", "Absent"])
    {
        if token.delta_line == 0 {
            character += token.delta_start;
        } else {
            line += token.delta_line;
            character = token.delta_start;
        }
        let offset = text.find(name).unwrap();
        let before = &text[..offset];
        assert_eq!(
            line,
            before.bytes().filter(|byte| *byte == b'\n').count() as u32
        );
        assert_eq!(
            character,
            before.rsplit('\n').next().unwrap().encode_utf16().count() as u32
        );
        assert_eq!(token.length, name.encode_utf16().count() as u32);
        assert_eq!(token.token_type, 0);
        assert_eq!(token.token_modifiers_bitset, 0);
    }

    server.notify(Notification::new(
        "textDocument/didChange".into(),
        json!({
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": "class Example { int value; }"}]
        }),
    ));
    let response = request(
        &mut server,
        "textDocument/semanticTokens/full",
        json!({"textDocument": {"uri": uri}}),
    );
    assert_eq!(response.result.unwrap()["data"], json!([]));
}

#[test]
fn missing_documents_and_invalid_parameters_are_handled() {
    let (_root, mut server) = server();
    let response = request(
        &mut server,
        "textDocument/semanticTokens/full",
        json!({"textDocument": {"uri": "untitled:missing"}}),
    );
    assert!(response.result.unwrap().is_null());
    let response = request(&mut server, "textDocument/semanticTokens/full", json!({}));
    assert_eq!(response.error.unwrap().code, -32602);
}
