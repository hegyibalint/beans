use super::*;
use beans_core::location::Location;
use beans_testing::{TempDir, template::Template};
use lsp_server::Notification;
use serde_json::json;
use std::fs;
use url::Url;

#[test]
fn definition_request_returns_a_utf16_target_location() {
    let (_root, mut server) = server();
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
fn definition_link_uses_the_source_range_when_the_client_supports_it() {
    let root = TempDir::new();
    let mut params = initialize_params(root.root());
    params["capabilities"] = json!({"textDocument": {"definition": {"linkSupport": true}}});
    let mut server = Client::default();
    let initialized = request(&mut server, "initialize", params);
    assert!(initialized.error.is_none());

    let uri = "untitled:Example.java";
    let fixture = Template::parse(
        "class Café /*😀*/ { class <span>Box</span><T> {} <span>Box<cur><String></span> field; }",
    );
    server.notify(Notification::new(
        "textDocument/didOpen".into(),
        json!({"textDocument": {
            "uri": uri, "languageId": "java", "version": 1, "text": fixture.content
        }}),
    ));

    let response = request(
        &mut server,
        "textDocument/definition",
        json!({"textDocument": {"uri": uri},
               "position": position(&fixture, fixture.cursors[0].offset)}),
    );
    let range = |index: usize| {
        let span = &fixture.spans[index];
        json!({
            "start": position(&fixture, span.start),
            "end": position(&fixture, span.end)
        })
    };
    assert_eq!(
        response.result,
        Some(json!([{
            "originSelectionRange": range(1),
            "targetUri": uri,
            "targetRange": range(0),
            "targetSelectionRange": range(0)
        }]))
    );
}

#[test]
fn definition_location_reads_unopened_external_file_text_from_disk() {
    let root = TempDir::new();
    let project = root.path("project");
    fs::create_dir(&project).unwrap();
    let sources = project.join("src");
    let target = Template::parse("package library;\n/* 😀 */ public class <span>Widget</span> {}");
    let target_path = root.write("library/Widget.java", &target.content);
    let mut engine = beans_engine::Engine::new(&project).unwrap();
    engine
        .process(Location::file(&target_path), &target.content)
        .unwrap();
    drop(engine);

    let mut server = Client::default();
    let root_uri = Url::from_directory_path(&project).unwrap().to_string();
    let initialized = request(
        &mut server,
        "initialize",
        json!({"capabilities": {}, "workspaceFolders": [{"uri": root_uri, "name": "project"}]}),
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

    let target_uri = Url::from_file_path(&target_path).unwrap().to_string();
    let expected = json!({
        "uri": target_uri,
        "range": {
            "start": position(&target, target.spans[0].start),
            "end": position(&target, target.spans[0].end)
        }
    });
    assert_eq!(response.result, Some(expected));
}

#[test]
fn goto_definition_without_an_open_document_returns_null() {
    let (_root, mut server) = server();
    let response = request(
        &mut server,
        "textDocument/definition",
        json!({"textDocument": {"uri": "untitled:Example.java"},
               "position": {"line": 0, "character": 0}}),
    );

    assert_eq!(response.result, Some(Value::Null));
}

#[test]
fn invalid_goto_definition_parameters_are_rejected() {
    let (_root, mut server) = server();
    assert_error(
        request(&mut server, "textDocument/definition", Value::Null),
        ErrorCode::InvalidParams,
    );
}
