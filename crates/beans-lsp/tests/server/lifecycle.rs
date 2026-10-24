use std::{fs, io};

use beans_testing::TempDir;
use lsp_server::Notification;
use serde_json::json;

use super::*;

#[test]
fn initialization_advertises_supported_capabilities_and_creates_the_cache() {
    let root = TempDir::new();
    let response = request(
        &mut Client::default(),
        "initialize",
        initialize_params(root.root()),
    );
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert_eq!(
        result["capabilities"],
        json!({
            "definitionProvider": true, "hoverProvider": true,
            "semanticTokensProvider": {"legend": {"tokenTypes": ["type"], "tokenModifiers": []}, "full": true},
            "textDocumentSync": {"openClose": true, "change": 1}
        })
    );
    assert_eq!(result["serverInfo"]["name"], "beans");
    assert!(root.path(".beans/models").is_file());
}

#[test]
fn requests_before_initialization_are_rejected() {
    for method in ["shutdown", "textDocument/hover"] {
        assert_error(
            request(&mut Client::default(), method, Value::Null),
            ErrorCode::ServerNotInitialized,
        );
    }
}

#[test]
fn notifications_before_initialization_are_ignored() {
    let root = TempDir::new();
    let mut client = Client::default();
    assert!(client.notify(Notification::new("textDocument/didOpen".into(), json!({
        "textDocument": {"uri": "untitled:Example.java", "languageId": "java", "version": 1, "text": "class C extends Target {}"}
    }))).is_empty());
    assert!(!root.path(".beans").exists());
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
    assert_eq!(request(&mut client, "textDocument/hover", json!({
        "textDocument": {"uri": "untitled:Example.java"}, "position": {"line": 0, "character": 16}
    })).result, Some(Value::Null));
}

#[test]
fn invalid_initialization_allows_a_later_valid_attempt() {
    let root = TempDir::new();
    let mut client = Client::default();
    assert_error(
        request(&mut client, "initialize", Value::Null),
        ErrorCode::InvalidParams,
    );
    assert_error(
        request(&mut client, "textDocument/hover", Value::Null),
        ErrorCode::ServerNotInitialized,
    );
    assert!(!root.path(".beans").exists());
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
    assert!(root.path(".beans/models").is_file());
}

#[test]
fn unsupported_workspaces_fail_initialization_without_creating_a_cache() {
    let root = TempDir::new();
    let single = initialize_params(root.root());
    let folder = single["workspaceFolders"][0].clone();
    let mut client = Client::default();
    for params in [
        json!({"capabilities": {}}),
        json!({"capabilities": {}, "workspaceFolders": []}),
        json!({"capabilities": {}, "workspaceFolders": [folder.clone(), folder]}),
        json!({"capabilities": {}, "workspaceFolders": [{"uri": "ssh://host/project", "name": "remote"}]}),
    ] {
        assert_error(
            request(&mut client, "initialize", params),
            ErrorCode::RequestFailed,
        );
        assert_error(
            request(&mut client, "textDocument/hover", Value::Null),
            ErrorCode::ServerNotInitialized,
        );
        assert!(!root.path(".beans").exists());
    }
    assert!(request(&mut client, "initialize", single).error.is_none());
}

#[test]
fn cache_errors_fail_initialization_and_allow_retry() {
    let root = TempDir::new();
    let path = root.write(".beans", "blocked");
    let mut client = Client::default();
    let error = request(&mut client, "initialize", initialize_params(root.root()))
        .error
        .unwrap();
    assert_eq!(error.code, ErrorCode::RequestFailed as i32);
    assert!(error.message.contains(&path.display().to_string()));
    assert_error(
        request(&mut client, "textDocument/hover", Value::Null),
        ErrorCode::ServerNotInitialized,
    );

    fs::remove_file(path).unwrap();
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
    assert!(root.path(".beans/models").is_file());
}

#[test]
fn initialization_cannot_be_repeated() {
    let (_root, mut client) = server();
    assert_error(
        request(&mut client, "initialize", json!({"capabilities": {}})),
        ErrorCode::InvalidRequest,
    );
}

#[test]
fn unsupported_requests_receive_method_not_found() {
    let (_root, mut client) = server();
    for method in ["textDocument/declaration", "$/unknown"] {
        assert_error(
            request(&mut client, method, Value::Null),
            ErrorCode::MethodNotFound,
        );
    }
}

#[test]
fn shutdown_ignores_parameters_rejects_requests_and_exit_succeeds() {
    let (_root, mut client) = server();
    let response = request(&mut client, "shutdown", json!({"ignored": true}));
    assert!(response.error.is_none());
    assert_eq!(response.result, Some(Value::Null));
    for method in ["shutdown", "initialize", "textDocument/hover"] {
        assert_error(
            request(&mut client, method, Value::Null),
            ErrorCode::InvalidRequest,
        );
    }
    client.exit().unwrap();
}

#[test]
fn exit_without_shutdown_is_an_error() {
    assert!(Client::default().exit().is_err());
}

#[test]
fn disconnect_without_exit_is_an_error() {
    assert_eq!(
        Client::default().disconnect().unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
}
