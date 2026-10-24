use lsp_server::Notification;
use serde_json::json;
use url::Url;

use super::*;

const DESCRIPTOR: &str = include_str!("../../../../examples/playground/beans.toml");
const EXAMPLE: &str = include_str!("../../../../examples/playground/src/demo/Example.java");
const WIDGET: &str = include_str!("../../../../examples/playground/src/library/Widget.java");

#[test]
fn definition_queries_are_unrestricted_across_descriptor_units() {
    for depends_on_library in [true, false] {
        let root = TempDir::new();
        root.write("src/demo/Example.java", EXAMPLE);
        root.write("src/library/Widget.java", WIDGET);
        let descriptor = if depends_on_library {
            DESCRIPTOR.to_owned()
        } else {
            DESCRIPTOR.replace("depends_on = [\"library\"]", "")
        };
        root.write("beans.toml", descriptor);
        let mut client = Client::default();
        assert!(
            request(&mut client, "initialize", initialize_params(root.root()))
                .error
                .is_none()
        );
        let uri = Url::from_file_path(root.path("src/demo/Example.java")).unwrap();
        client.notify(Notification::new("textDocument/didOpen".into(), json!({
            "textDocument": {"uri": uri.as_str(), "languageId": "java", "version": 1, "text": EXAMPLE}
        })));
        let fixture = Template::parse(EXAMPLE);
        let response = request(
            &mut client,
            "textDocument/definition",
            json!({
                "textDocument": {"uri": uri.as_str()},
                "position": position(&fixture, EXAMPLE.find("Widget imported").unwrap())
            }),
        );
        let target = Template::parse(WIDGET);
        let start = WIDGET.find("Widget").unwrap();
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
