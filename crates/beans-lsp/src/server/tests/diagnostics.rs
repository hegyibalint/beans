use lsp_server::Notification;
use lsp_types::PublishDiagnosticsParams;
use serde_json::{Value, json};

use super::{Lifecycle, server};
use crate::server::Server;

fn notify(server: &mut Server, method: &str, params: Value) -> Vec<PublishDiagnosticsParams> {
    server
        .notify(Notification::new(method.into(), params))
        .into_iter()
        .map(|notification| {
            assert_eq!(notification.method, "textDocument/publishDiagnostics");
            serde_json::from_value(notification.params).unwrap()
        })
        .collect()
}

fn open(server: &mut Server, uri: &str, text: &str) -> Vec<PublishDiagnosticsParams> {
    notify(
        server,
        "textDocument/didOpen",
        json!({
            "textDocument": {"uri": uri, "languageId": "java", "version": 1, "text": text}
        }),
    )
}

#[test]
fn open_change_and_close_publish_versioned_utf16_diagnostics() {
    let mut server = server(Lifecycle::Running);
    let uri = "untitled:Example.java";
    let text = "class Example { /* 😀 */ Missing value; }";
    let publications = open(&mut server, uri, text);
    assert_eq!(publications.len(), 1);
    let publication = &publications[0];
    assert_eq!(publication.uri.as_str(), uri);
    assert_eq!(publication.version, Some(1));
    assert_eq!(publication.diagnostics.len(), 1);
    let diagnostic = &publication.diagnostics[0];
    let start = text[..text.find("Missing").unwrap()].encode_utf16().count() as u32;
    assert_eq!(diagnostic.range.start, lsp_types::Position::new(0, start));
    assert_eq!(diagnostic.range.end, lsp_types::Position::new(0, start + 7));
    assert_eq!(
        diagnostic.severity,
        Some(lsp_types::DiagnosticSeverity::ERROR)
    );
    assert_eq!(diagnostic.source.as_deref(), Some("beans"));
    assert_eq!(
        diagnostic.code,
        Some(lsp_types::NumberOrString::String(
            "cannot-find-symbol".into()
        ))
    );

    let publications = notify(
        &mut server,
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": uri, "version": 2},
            "contentChanges": [{"text": "class Example { int value; }"}]
        }),
    );
    assert_eq!(publications[0].version, Some(2));
    assert!(publications[0].diagnostics.is_empty());

    let publications = notify(
        &mut server,
        "textDocument/didClose",
        json!({
            "textDocument": {"uri": uri}
        }),
    );
    assert_eq!(publications.len(), 1);
    assert_eq!(publications[0].uri.as_str(), uri);
    assert_eq!(publications[0].version, None);
    assert!(publications[0].diagnostics.is_empty());
}

#[test]
fn changes_in_another_document_refresh_open_dependents() {
    let mut server = server(Lifecycle::Running);
    let uri = "untitled:Example.java";
    assert_eq!(
        open(&mut server, uri, "class Example { Missing value; }")[0]
            .diagnostics
            .len(),
        1
    );
    let publications = open(&mut server, "untitled:Missing.java", "class Missing {}");
    assert_eq!(publications.len(), 2);
    assert!(
        publications
            .iter()
            .all(|publication| publication.diagnostics.is_empty())
    );

    let publications = notify(
        &mut server,
        "textDocument/didClose",
        json!({
            "textDocument": {"uri": "untitled:Missing.java"}
        }),
    );
    let dependent = publications
        .iter()
        .find(|publication| publication.uri.as_str() == uri)
        .unwrap();
    assert_eq!(dependent.diagnostics.len(), 1);
}

#[test]
fn rejected_changes_and_other_languages_do_not_publish() {
    let mut server = server(Lifecycle::Running);
    let uri = "untitled:Example.java";
    open(&mut server, uri, "class Example { Missing value; }");
    assert!(
        notify(
            &mut server,
            "textDocument/didChange",
            json!({
                "textDocument": {"uri": uri, "version": 1},
                "contentChanges": [{"text": "class Example {}"}]
            })
        )
        .is_empty()
    );
    assert!(notify(&mut server, "textDocument/didChange", json!({
        "textDocument": {"uri": uri, "version": 2},
        "contentChanges": [{"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}}, "text": "x"}]
    })).is_empty());
    assert!(notify(&mut server, "textDocument/didOpen", json!({
        "textDocument": {"uri": "untitled:notes", "languageId": "plaintext", "version": 1, "text": "Missing"}
    })).is_empty());
}
