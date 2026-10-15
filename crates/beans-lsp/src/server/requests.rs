use lsp_server::{ErrorCode, Request, Response};
use lsp_types::{
    GotoDefinitionParams, HoverParams, InitializeParams, InitializeResult, ServerCapabilities,
    ServerInfo, TextDocumentSyncKind, TextDocumentSyncOptions, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};

use super::{Lifecycle, Server};
use crate::features::InitializeError;

impl Server {
    pub(super) fn respond(&mut self, request: Request) -> Result<Response, String> {
        match (&self.lifecycle, request.method.as_str()) {
            (Lifecycle::Uninitialized, "initialize") => {
                let params = match serde_json::from_value::<InitializeParams>(request.params) {
                    Ok(params) => params,
                    Err(error) => {
                        return Ok(Response::new_err(
                            request.id,
                            ErrorCode::InvalidParams as i32,
                            error.to_string(),
                        ));
                    }
                };
                match self.features.initialize(&params) {
                    Ok(()) => {}
                    Err(InitializeError::Workspace(error)) => {
                        return Ok(Response::new_err(
                            request.id,
                            ErrorCode::RequestFailed as i32,
                            error,
                        ));
                    }
                    Err(InitializeError::Storage(error)) => {
                        return Err(format!("cannot start Engine storage: {error}"));
                    }
                }
                self.lifecycle = Lifecycle::Running;
                Ok(Response::new_ok(
                    request.id,
                    InitializeResult {
                        capabilities: ServerCapabilities {
                            hover_provider: Some(true.into()),
                            definition_provider: Some(lsp_types::OneOf::Left(true)),
                            workspace: Some(WorkspaceServerCapabilities {
                                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                                    supported: Some(true),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }),
                            text_document_sync: Some(
                                TextDocumentSyncOptions {
                                    open_close: Some(true),
                                    change: Some(TextDocumentSyncKind::FULL),
                                    ..Default::default()
                                }
                                .into(),
                            ),
                            ..Default::default()
                        },
                        server_info: Some(ServerInfo {
                            name: "beans".into(),
                            version: Some(env!("CARGO_PKG_VERSION").into()),
                        }),
                    },
                ))
            }
            (Lifecycle::Uninitialized, _) => Ok(Response::new_err(
                request.id,
                ErrorCode::ServerNotInitialized as i32,
                "server is not initialized".into(),
            )),
            (Lifecycle::Shutdown, _) => Ok(Response::new_err(
                request.id,
                ErrorCode::InvalidRequest as i32,
                "server has shut down".into(),
            )),
            (Lifecycle::Running, "initialize") => Ok(Response::new_err(
                request.id,
                ErrorCode::InvalidRequest as i32,
                "server is already initialized".into(),
            )),
            (Lifecycle::Running, "shutdown") => {
                self.lifecycle = Lifecycle::Shutdown;
                Ok(Response::new_ok(request.id, ()))
            }
            (Lifecycle::Running, "textDocument/hover") => {
                let params = match serde_json::from_value::<HoverParams>(request.params) {
                    Ok(params) => params,
                    Err(error) => {
                        return Ok(Response::new_err(
                            request.id,
                            ErrorCode::InvalidParams as i32,
                            error.to_string(),
                        ));
                    }
                };
                Ok(match self.features.hover(&params) {
                    Ok(hover) => Response::new_ok(request.id, hover),
                    Err(error) => {
                        Response::new_err(request.id, ErrorCode::RequestFailed as i32, error)
                    }
                })
            }
            (Lifecycle::Running, "textDocument/definition") => {
                let params = match serde_json::from_value::<GotoDefinitionParams>(request.params) {
                    Ok(params) => params,
                    Err(error) => {
                        return Ok(Response::new_err(
                            request.id,
                            ErrorCode::InvalidParams as i32,
                            error.to_string(),
                        ));
                    }
                };
                Ok(match self.features.goto_definition(&params) {
                    Ok(definition) => Response::new_ok(request.id, definition),
                    Err(error) => {
                        Response::new_err(request.id, ErrorCode::RequestFailed as i32, error)
                    }
                })
            }
            (Lifecycle::Running, _) => Ok(Response::new_err(
                request.id,
                ErrorCode::MethodNotFound as i32,
                "unsupported method".into(),
            )),
        }
    }
}
