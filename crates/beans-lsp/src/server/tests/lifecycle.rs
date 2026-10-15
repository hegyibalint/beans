use super::*;
use lsp_server::{Connection, Notification};
use serde_json::json;
use std::io;

#[test]
fn initialization_advertises_open_and_close_synchronization() {
    let root = tempfile::tempdir().unwrap();
    let response = request(
        &mut Server::default(),
        "initialize",
        initialize_params(root.path()),
    );
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert_eq!(
        result["capabilities"],
        json!({"definitionProvider": true, "hoverProvider": true,
               "semanticTokensProvider": {"legend": {"tokenTypes": ["type"], "tokenModifiers": []}, "full": true},
               "textDocumentSync": {"openClose": true, "change": 1}})
    );
    assert_eq!(result["serverInfo"]["name"], "beans");
    assert!(root.path().join(".beans/models").is_file());
}

#[test]
fn requests_before_initialization_are_rejected() {
    for method in ["shutdown", "textDocument/hover"] {
        assert_error(
            request(&mut Server::default(), method, Value::Null),
            ErrorCode::ServerNotInitialized,
        );
    }
}

#[test]
fn notifications_before_initialization_do_not_create_an_engine() {
    let mut server = Server::default();
    assert!(server.notify(Notification::new("textDocument/didOpen".into(), json!({
        "textDocument": {"uri": "untitled:Example.java", "languageId": "java", "version": 1, "text": "class Example {}"}
    }))).is_empty());
    assert!(server.features.is_none());
}

#[test]
fn invalid_initialization_does_not_start_the_server() {
    let mut server = Server::default();
    assert_error(
        request(&mut server, "initialize", Value::Null),
        ErrorCode::InvalidParams,
    );
    assert!(matches!(server.lifecycle, Lifecycle::Uninitialized));
    assert!(server.features.is_none());
}

#[test]
fn unsupported_workspaces_fail_initialization_without_creating_a_cache() {
    let root = tempfile::tempdir().unwrap();
    let single = initialize_params(root.path());
    let folder = single["workspaceFolders"][0].clone();
    for params in [
        json!({"capabilities": {}}),
        json!({"capabilities": {}, "workspaceFolders": []}),
        json!({"capabilities": {}, "workspaceFolders": [folder.clone(), folder]}),
        json!({"capabilities": {}, "workspaceFolders": [{"uri": "ssh://host/project", "name": "remote"}]}),
    ] {
        let mut server = Server::default();
        assert_error(
            request(&mut server, "initialize", params),
            ErrorCode::RequestFailed,
        );
        assert!(matches!(server.lifecycle, Lifecycle::Uninitialized));
        assert!(server.features.is_none());
        assert!(!root.path().join(".beans").exists());
    }
}

#[test]
fn cache_errors_fail_initialization_without_starting_the_server() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(".beans");
    std::fs::write(&path, "blocked").unwrap();
    let mut server = Server::default();
    let response = request(&mut server, "initialize", initialize_params(root.path()));
    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::RequestFailed as i32);
    assert!(error.message.contains(&path.display().to_string()));
    assert!(matches!(server.lifecycle, Lifecycle::Uninitialized));
    assert!(server.features.is_none());
}

#[test]
fn initialization_cannot_be_repeated() {
    let (_root, mut server) = server();
    assert_error(
        request(&mut server, "initialize", json!({"capabilities": {}})),
        ErrorCode::InvalidRequest,
    );
}

#[test]
fn unsupported_requests_receive_method_not_found() {
    let (_root, mut server) = server();
    for method in ["textDocument/declaration", "$/unknown"] {
        assert_error(
            request(&mut server, method, Value::Null),
            ErrorCode::MethodNotFound,
        );
    }
}

#[test]
fn shutdown_returns_null_and_rejects_further_requests() {
    let (_root, mut server) = server();
    let response = request(&mut server, "shutdown", Value::Null);
    assert!(response.error.is_none());
    assert_eq!(response.result, Some(Value::Null));
    for method in ["shutdown", "initialize", "textDocument/hover"] {
        assert_error(
            request(&mut server, method, Value::Null),
            ErrorCode::InvalidRequest,
        );
    }
}

#[test]
fn shutdown_ignores_parameters() {
    let (_root, mut server) = server();
    let response = request(&mut server, "shutdown", json!({"ignored": true}));

    assert!(response.error.is_none());
    assert!(matches!(server.lifecycle, Lifecycle::Shutdown));
}

#[test]
fn exit_without_shutdown_is_an_error() {
    let (server, client) = Connection::memory();
    client
        .sender
        .send(Notification::new("exit".into(), ()).into())
        .unwrap();
    assert!(crate::run(server).is_err());
}

#[test]
fn disconnect_without_exit_is_an_error() {
    let (server, client) = Connection::memory();
    drop(client);
    assert_eq!(
        crate::run(server).unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
}
