use super::*;
use beans_testing::template::Template;
use lsp_server::Notification;
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

#[test]
fn field_type_navigates_to_its_declaration_with_utf16_positions() {
    let mut server = server(Lifecycle::Running);
    let uri = "untitled:Example.java";
    let fixture =
        Template::parse("class Café /*😀*/ { class <span>Target</span> {} <cur>Target field; }");
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": uri, "languageId": "java", "version": 1, "text": fixture.content
        }}),
    ));
    let target = &fixture.spans[0];
    let use_position = position(&fixture, fixture.cursors[0].offset);

    let response = request(
        &mut server,
        "textDocument/definition",
        json!({"textDocument": {"uri": uri},
               "position": use_position}),
    );

    assert_eq!(
        response.result,
        Some(json!({
            "uri": uri,
            "range": {
                "start": position(&fixture, target.start),
                "end": position(&fixture, target.end)
            }
        }))
    );
}

#[test]
fn imported_field_type_navigates_to_an_unopened_workspace_source_file() {
    let root = std::env::temp_dir().join(format!(
        "beans-definition-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let sources = root.join("src");
    let target_path = root.join("library/Widget.java");
    fs::create_dir_all(target_path.parent().unwrap()).unwrap();
    let target = Template::parse("package library;\npublic class <span>Widget</span> {}");
    fs::write(&target_path, &target.content).unwrap();

    let mut server = Server::default();
    let root_uri = Url::from_directory_path(&root).unwrap().to_string();
    let initialized = request(
        &mut server,
        "initialize",
        json!({"capabilities": {}, "workspaceFolders": [{"uri": root_uri, "name": "playground"}]}),
    );
    assert!(initialized.error.is_none());

    let use_uri = Url::from_file_path(sources.join("demo/Example.java"))
        .unwrap()
        .to_string();
    let use_fixture = Template::parse(
        "package demo;\nimport library.Widget;\nclass Example { <cur>Widget imported; }",
    );
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": use_uri, "languageId": "java", "version": 1, "text": use_fixture.content
        }}),
    ));
    let use_position = position(&use_fixture, use_fixture.cursors[0].offset);
    let response = request(
        &mut server,
        "textDocument/definition",
        json!({"textDocument": {"uri": use_uri},
               "position": use_position}),
    );

    assert_eq!(
        response.result,
        Some(json!({
            "uri": Url::from_file_path(target_path).unwrap().to_string(),
            "range": {
                "start": position(&target, target.spans[0].start),
                "end": position(&target, target.spans[0].end)
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
