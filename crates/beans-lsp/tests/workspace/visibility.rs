use lsp_server::Notification;
use serde_json::json;
use url::Url;

use super::*;

#[test]
fn definition_queries_are_unrestricted_across_descriptor_units() {
    let example = "package demo; import library.Widget; class Example { Widget imported; }";
    let widget = "package library; public class Widget {}";
    for depends_on_library in [true, false] {
        let root = TempDir::new();
        root.write("src/demo/Example.java", example);
        root.write("src/library/Widget.java", widget);
        let dependency = if depends_on_library {
            "depends_on = ['library']\n"
        } else {
            ""
        };
        root.write(
            "beans.toml",
            format!(
                "[unit.demo]\nsources = ['src/demo']\n{dependency}[unit.library]\nsources = ['src/library']\n"
            ),
        );
        let mut client = Client::default();
        assert!(
            request(&mut client, "initialize", initialize_params(root.root()))
                .error
                .is_none()
        );
        let uri = Url::from_file_path(root.path("src/demo/Example.java")).unwrap();
        client.notify(Notification::new("textDocument/didOpen".into(), json!({
            "textDocument": {"uri": uri.as_str(), "languageId": "java", "version": 1, "text": example}
        })));
        let fixture = Template::parse(example);
        let response = request(
            &mut client,
            "textDocument/definition",
            json!({
                "textDocument": {"uri": uri.as_str()},
                "position": position(&fixture, example.find("Widget imported").unwrap())
            }),
        );
        let target = Template::parse(widget);
        let start = widget.find("Widget").unwrap();
        let expected = json!({
            "uri": Url::from_file_path(root.path("src/library/Widget.java")).unwrap().as_str(),
            "range": {"start": position(&target, start), "end": position(&target, start + "Widget".len())}
        });
        assert_eq!(
            response.result,
            Some(expected),
            "depends_on library: {depends_on_library}"
        );
    }
}

#[test]
fn invalid_descriptors_fail_before_cache_creation_and_can_be_repaired() {
    let root = TempDir::new();
    let descriptor = root.write("beans.toml", "[unit.main]\ndepends_on = ['missing']\n");
    let mut client = Client::default();
    let response = request(&mut client, "initialize", initialize_params(root.root()));
    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::RequestFailed as i32);
    assert!(error.message.contains(&descriptor.display().to_string()));
    assert!(!root.path(".beans").exists());
    assert_error(
        request(&mut client, "textDocument/hover", serde_json::Value::Null),
        ErrorCode::ServerNotInitialized,
    );

    root.write("beans.toml", "[unit.main]\n");
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
    assert!(root.path(".beans/models").is_file());
}
