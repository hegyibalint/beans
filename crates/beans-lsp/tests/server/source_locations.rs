use lsp_server::Notification;
use serde_json::json;

use super::{request, server};

#[test]
fn file_and_virtual_documents_resolve_open_targets_and_preserve_uri_spelling() {
    for use_uri in ["file:///src/Use.java", "BEANS-jvm:///library/Use.class"] {
        let (_root, mut client) = server();
        let target_uri = "file:///library/%54arget.java";
        let text = "import library.Target; class Use { Target field; }";
        let target = "package library; public class Target {}";
        for (uri, contents) in [(use_uri, text), (target_uri, target)] {
            client.notify(Notification::new("textDocument/didOpen".into(), json!({
                "textDocument": {"uri": uri, "languageId": "java", "version": 1, "text": contents}
            })));
        }
        let params = json!({
            "textDocument": {"uri": use_uri},
            "position": {"line": 0, "character": text.rfind("Target").unwrap()}
        });
        let definition = request(&mut client, "textDocument/definition", params.clone());
        let start = target.find("Target").unwrap();
        assert_eq!(
            definition.result,
            Some(json!({
                "uri": target_uri,
                "range": {"start": {"line": 0, "character": start}, "end": {"line": 0, "character": start + "Target".len()}}
            }))
        );
        let hover = request(&mut client, "textDocument/hover", params);
        assert!(hover.error.is_none());
        assert!(!hover.result.unwrap().is_null());
    }
}
