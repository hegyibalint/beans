use std::fs;

use lsp_server::Notification;
use serde_json::json;
use url::Url;

use super::*;

const DESCRIPTOR: &str = include_str!("../../../../../examples/playground/beans.toml");
const EXAMPLE: &str = include_str!("../../../../../examples/playground/src/demo/Example.java");
const WIDGET: &str = include_str!("../../../../../examples/playground/src/library/Widget.java");

#[test]
fn descriptor_dependencies_control_definition_queries_after_restart() {
    for depends_on_library in [true, false] {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/demo")).unwrap();
        fs::create_dir_all(root.path().join("src/library")).unwrap();
        fs::write(root.path().join("src/demo/Example.java"), EXAMPLE).unwrap();
        fs::write(root.path().join("src/library/Widget.java"), WIDGET).unwrap();
        let descriptor = if depends_on_library {
            DESCRIPTOR.to_owned()
        } else {
            DESCRIPTOR.replace("depends_on = [\"library\"]", "")
        };
        fs::write(root.path().join("beans.toml"), descriptor).unwrap();
        let root_uri = Url::from_directory_path(root.path()).unwrap();
        let mut server = Server::default();
        let response = request(
            &mut server,
            "initialize",
            json!({
                "capabilities": {},
                "workspaceFolders": [{"uri": root_uri.as_str(), "name": "playground"}],
            }),
        );
        assert!(response.error.is_none());
        let uri = Url::from_file_path(root.path().join("src/demo/Example.java")).unwrap();
        server.notify(Notification::new("textDocument/didOpen".into(), json!({
            "textDocument": {"uri": uri.as_str(), "languageId": "java", "version": 1, "text": EXAMPLE}
        })));
        let fixture = Template::parse(EXAMPLE);

        let response = request(
            &mut server,
            "textDocument/definition",
            json!({
                "textDocument": {"uri": uri.as_str()},
                "position": position(&fixture, EXAMPLE.find("Widget imported").unwrap()),
            }),
        );

        let expected = if depends_on_library {
            let target = Template::parse(WIDGET);
            let start = WIDGET.find("Widget").unwrap();
            json!({
                "uri": Url::from_file_path(root.path().join("src/library/Widget.java")).unwrap().as_str(),
                "range": {"start": position(&target, start), "end": position(&target, start + "Widget".len())},
            })
        } else {
            Value::Null
        };
        assert_eq!(
            response.result,
            Some(expected),
            "depends_on library: {depends_on_library}"
        );
    }
}

#[test]
fn invalid_descriptors_fail_initialization_with_the_descriptor_path() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("beans.toml"),
        "[unit.main]\ndepends_on = [\"missing\"]\n",
    )
    .unwrap();
    let mut server = Server::default();
    let uri = Url::from_directory_path(root.path()).unwrap();

    let response = request(
        &mut server,
        "initialize",
        json!({
            "capabilities": {}, "workspaceFolders": [{"uri": uri.as_str(), "name": "project"}]
        }),
    );

    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::RequestFailed as i32);
    assert!(
        error
            .message
            .contains(&root.path().join("beans.toml").display().to_string())
    );
    assert!(matches!(server.lifecycle, Lifecycle::Uninitialized));
}
