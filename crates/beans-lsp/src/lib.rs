//! LSP lifecycle and protocol dispatch around the Beans engine.

mod features;
mod open_document;

use std::{collections::HashMap, io};

use beans_engine::Engine;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::{
    HoverParams, InitializeParams, InitializeResult, ServerCapabilities, ServerInfo,
    TextDocumentSyncKind, TextDocumentSyncOptions,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    },
    request::{HoverRequest, Request as _},
};
use open_document::OpenDocument;

#[derive(Default)]
enum Lifecycle {
    #[default]
    Uninitialized,
    Running,
    Shutdown,
}

/// The LSP transport and the application engine it serves.
#[derive(Default)]
pub struct Server {
    lifecycle: Lifecycle,
    engine: Engine,
    open_documents: HashMap<String, OpenDocument>,
}

impl Server {
    /// Serves a connection until `exit`.
    ///
    /// An exit without shutdown, a disconnected client, or a failed send is an error.
    /// Lifecycle rules: <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initialize>.
    pub fn run(mut self, connection: Connection) -> io::Result<()> {
        for message in &connection.receiver {
            match message {
                Message::Request(request) => {
                    let response = self.respond(request);
                    connection
                        .sender
                        .send(response.into())
                        .map_err(io::Error::other)?;
                }
                Message::Notification(notification) if notification.method == "exit" => {
                    // LSP #exit requires failure unless shutdown was received first.
                    return match self.lifecycle {
                        Lifecycle::Shutdown => Ok(()),
                        _ => Err(io::Error::other("exit received before shutdown")),
                    };
                }
                Message::Notification(notification) => self.notify(notification),
                // There are no outgoing requests, so responses need no dispatch yet.
                Message::Response(_) => {}
            }
        }
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "client disconnected before exit",
        ))
    }

    fn respond(&mut self, request: Request) -> Response {
        let error = match (&self.lifecycle, request.method.as_str()) {
            (Lifecycle::Uninitialized, "initialize") => {
                match serde_json::from_value::<InitializeParams>(request.params) {
                    Ok(_) => {
                        self.lifecycle = Lifecycle::Running;
                        return Response::new_ok(
                            request.id,
                            InitializeResult {
                                capabilities: ServerCapabilities {
                                    hover_provider: Some(true.into()),
                                    text_document_sync: Some(
                                        TextDocumentSyncOptions {
                                            open_close: Some(true),
                                            change: Some(TextDocumentSyncKind::FULL),
                                            ..TextDocumentSyncOptions::default()
                                        }
                                        .into(),
                                    ),
                                    ..ServerCapabilities::default()
                                },
                                server_info: Some(ServerInfo {
                                    name: "beans".into(),
                                    version: Some(env!("CARGO_PKG_VERSION").into()),
                                }),
                            },
                        );
                    }
                    Err(error) => {
                        return Response::new_err(
                            request.id,
                            ErrorCode::InvalidParams as i32,
                            error.to_string(),
                        );
                    }
                }
            }
            (Lifecycle::Uninitialized, _) => {
                (ErrorCode::ServerNotInitialized, "server is not initialized")
            }
            // LSP #shutdown requires InvalidRequest for all subsequent requests.
            (Lifecycle::Shutdown, _) => (ErrorCode::InvalidRequest, "server has shut down"),
            (Lifecycle::Running, "initialize") => {
                (ErrorCode::InvalidRequest, "server is already initialized")
            }
            (Lifecycle::Running, "shutdown") => {
                self.lifecycle = Lifecycle::Shutdown;
                return Response::new_ok(request.id, ());
            }
            (Lifecycle::Running, HoverRequest::METHOD) => {
                return match serde_json::from_value::<HoverParams>(request.params) {
                    Ok(params) => Response::new_ok(
                        request.id,
                        features::hover(&self.engine, &self.open_documents, params),
                    ),
                    Err(error) => Response::new_err(
                        request.id,
                        ErrorCode::InvalidParams as i32,
                        error.to_string(),
                    ),
                };
            }
            (Lifecycle::Running, _) => (ErrorCode::MethodNotFound, "unsupported method"),
        };
        Response::new_err(request.id, error.0 as i32, error.1.into())
    }

    fn notify(&mut self, notification: Notification) {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(params) = notification.extract(DidOpenTextDocument::METHOD) {
                    features::text_document::did_open(
                        &mut self.engine,
                        &mut self.open_documents,
                        params,
                    );
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(params) = notification.extract(DidChangeTextDocument::METHOD) {
                    features::text_document::did_change(
                        &mut self.engine,
                        &mut self.open_documents,
                        params,
                    );
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(params) = notification.extract(DidCloseTextDocument::METHOD) {
                    features::text_document::did_close(
                        &mut self.engine,
                        &mut self.open_documents,
                        params,
                    );
                }
            }
            _ => {}
        }
    }
}

pub fn run(connection: Connection) -> io::Result<()> {
    Server::default().run(connection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_server::Notification;
    use serde_json::{Value, json};

    fn server(lifecycle: Lifecycle) -> Server {
        Server {
            lifecycle,
            ..Server::default()
        }
    }

    fn request(server: &mut Server, method: &str, params: Value) -> Response {
        let response = server.respond(Request::new(7.into(), method.into(), params));
        assert_eq!(response.id, 7.into());
        response
    }

    fn assert_error(response: Response, code: ErrorCode) {
        assert_eq!(response.error.unwrap().code, code as i32);
        assert!(response.result.is_none());
    }

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
            json!({"hoverProvider": true, "textDocumentSync": {"openClose": true, "change": 1}})
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
    fn hovering_over_a_type_identifier_uses_the_open_document_and_utf16_range() {
        let mut server = server(Lifecycle::Running);
        let uri = "untitled:Example.java";
        let text = "class Café /*😀*/ extends Box<Café> {}";
        server.notify(Notification::new(
            "textDocument/didOpen".into(),
            json!({"textDocument": {
                "uri": uri, "languageId": "java", "version": 1, "text": text
            }}),
        ));
        let hover_at = |server: &mut Server, character: u32| {
            request(
                server,
                "textDocument/hover",
                json!({"textDocument": {"uri": uri}, "position": {"line": 0, "character": character}}),
            )
        };
        let start = text[..text.find("Box").unwrap()].encode_utf16().count() as u32;
        let response = hover_at(&mut server, start);
        assert_eq!(
            response.result.unwrap(),
            json!({
                "contents": {"kind": "plaintext", "value": "Box<Café>"},
                "range": {"start": {"line": 0, "character": start},
                          "end": {"line": 0, "character": start + 3}}
            })
        );
        assert_eq!(hover_at(&mut server, start + 3).result, Some(Value::Null));
        assert_eq!(hover_at(&mut server, 6).result, Some(Value::Null));

        server.notify(Notification::new(
            "textDocument/didChange".into(),
            json!({"textDocument": {"uri": uri, "version": 2},
                   "contentChanges": [{"text": "class C extends NewType {}"}]}),
        ));
        assert_eq!(
            hover_at(&mut server, 16).result.unwrap()["contents"]["value"],
            "NewType"
        );
        server.notify(Notification::new(
            "textDocument/didClose".into(),
            json!({"textDocument": {"uri": uri}}),
        ));
        assert_eq!(hover_at(&mut server, 16).result, Some(Value::Null));
    }

    #[test]
    fn invalid_hover_parameters_are_rejected() {
        assert_error(
            request(
                &mut server(Lifecycle::Running),
                "textDocument/hover",
                Value::Null,
            ),
            ErrorCode::InvalidParams,
        );
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
        assert!(run(server).is_err());
    }

    #[test]
    fn disconnect_without_exit_is_an_error() {
        let (server, client) = Connection::memory();
        drop(client);
        assert_eq!(
            run(server).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }
}
