use super::*;
use lsp_server::{Connection, Notification};
use serde_json::json;
use std::io;

#[test]
fn initialization_advertises_open_and_close_synchronization() {
    let response = request(
        &mut Server::default(),
        "initialize",
        json!({"capabilities": {}}),
    );
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert_eq!(
        result["capabilities"],
        json!({"declarationProvider": true, "hoverProvider": true,
               "textDocumentSync": {"openClose": true, "change": 1}})
    );
    assert_eq!(result["serverInfo"]["name"], "beans");
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
fn invalid_initialization_does_not_start_the_server() {
    let mut server = Server::default();
    assert_error(
        request(&mut server, "initialize", Value::Null),
        ErrorCode::InvalidParams,
    );
    assert!(matches!(server.lifecycle, Lifecycle::Uninitialized));
}

#[test]
fn initialization_cannot_be_repeated() {
    assert_error(
        request(
            &mut server(Lifecycle::Running),
            "initialize",
            json!({"capabilities": {}}),
        ),
        ErrorCode::InvalidRequest,
    );
}

#[test]
fn unsupported_requests_receive_method_not_found() {
    for method in ["textDocument/definition", "$/unknown"] {
        assert_error(
            request(&mut server(Lifecycle::Running), method, Value::Null),
            ErrorCode::MethodNotFound,
        );
    }
}

#[test]
fn shutdown_returns_null_and_rejects_further_requests() {
    let mut server = server(Lifecycle::Running);
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
    let mut server = server(Lifecycle::Running);
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
