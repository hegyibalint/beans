use lsp_server::{ErrorCode, Request, Response};
use lsp_types::{
    GotoDefinitionParams, HoverParams, InitializeParams, InitializeResult, OneOf,
    ServerCapabilities, ServerInfo, TextDocumentSyncKind, TextDocumentSyncOptions,
    request::{GotoDefinition, HoverRequest, Request as _},
};

use super::{Lifecycle, Server};

impl Server {
    pub(super) fn respond(&mut self, request: Request) -> Response {
        let error = match (&self.lifecycle, request.method.as_str()) {
            (Lifecycle::Uninitialized, "initialize") => {
                match serde_json::from_value::<InitializeParams>(request.params) {
                    Ok(_) => {
                        self.lifecycle = Lifecycle::Running;
                        return Response::new_ok(
                            request.id,
                            InitializeResult {
                                capabilities: ServerCapabilities {
                                    definition_provider: Some(OneOf::Left(true)),
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
            (Lifecycle::Running, GotoDefinition::METHOD) => {
                return match serde_json::from_value::<GotoDefinitionParams>(request.params) {
                    Ok(params) => {
                        Response::new_ok(request.id, self.features.goto_definition(params))
                    }
                    Err(error) => Response::new_err(
                        request.id,
                        ErrorCode::InvalidParams as i32,
                        error.to_string(),
                    ),
                };
            }
            (Lifecycle::Running, HoverRequest::METHOD) => {
                return match serde_json::from_value::<HoverParams>(request.params) {
                    Ok(params) => Response::new_ok(request.id, self.features.hover(params)),
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
}
