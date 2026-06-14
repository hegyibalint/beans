//! tower-lsp backend — thin async shell over the worker actor.
//!
//! Per ADR-0018 the graph core is single-threaded; per ADR-0020 the LSP
//! is a leaf consumer. This file owns the wire-protocol mapping and
//! dispatches each request through [`crate::actor::WorkerHandle`]. The
//! request handlers in [`crate::actor`] convert protocol values to and
//! from the [`beans::Workspace`] facade, which owns workspace indexing
//! and resolution. LSP-shaped formatting (hover) lives in
//! [`crate::hover`].

use beans::Workspace;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::actor::{Cmd, WorkerHandle, spawn_worker};

/// Server-wide state: just the [`beans::Workspace`] facade. Owned
/// exclusively by the worker thread per ADR-0018; the LSP backend itself
/// holds only a [`WorkerHandle`] and the [`Client`]. All workspace
/// semantics — indexed roots, source text, import/package context — live
/// inside the facade, not here: the LSP no longer owns indexing or
/// resolution mechanics, only the protocol rim around them.
pub struct ServerState {
    pub workspace: Workspace,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            workspace: Workspace::new(),
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// LSP server. `Send + Sync` because both [`Client`] and
/// [`WorkerHandle`] (which wraps an `mpsc::Sender`) are; the
/// `!Send + !Sync` graph and registries live on the worker thread the
/// handle dispatches to.
pub struct BeanBackend {
    client: Client,
    worker: WorkerHandle,
}

impl BeanBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            worker: spawn_worker(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BeanBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let root = params.root_uri.and_then(|u| u.to_file_path().ok());
        // Index workspace synchronously on the worker so that the first
        // post-initialize request hits a populated graph.
        let count = self
            .worker
            .send(|reply| Cmd::Initialize { root, reply })
            .await
            .unwrap_or(0);
        self.client
            .log_message(MessageType::INFO, format!("Beans: Indexed {} files", count))
            .await;
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Beans LSP server initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        if let Some(diag) = self
            .worker
            .send(move |reply| Cmd::DidOpen { uri, text, reply })
            .await
        {
            self.client
                .publish_diagnostics(diag.uri, diag.diagnostics, None)
                .await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };
        let text = change.text;
        if let Some(diag) = self
            .worker
            .send(move |reply| Cmd::DidChange { uri, text, reply })
            .await
        {
            self.client
                .publish_diagnostics(diag.uri, diag.diagnostics, None)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(diag) = self
            .worker
            .send(move |reply| Cmd::DidSave { uri, reply })
            .await
        {
            self.client
                .publish_diagnostics(diag.uri, diag.diagnostics, None)
                .await;
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        Ok(self
            .worker
            .send(move |reply| Cmd::GotoDefinition { uri, pos, reply })
            .await
            .flatten())
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let pos = params.range.start;
        Ok(self
            .worker
            .send(move |reply| Cmd::CodeAction { uri, pos, reply })
            .await
            .flatten())
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        Ok(self
            .worker
            .send(move |reply| Cmd::References { uri, pos, reply })
            .await
            .flatten())
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        Ok(self
            .worker
            .send(move |reply| Cmd::Hover { uri, pos, reply })
            .await
            .flatten())
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        Ok(self
            .worker
            .send(move |reply| Cmd::DocumentSymbol { uri, reply })
            .await
            .flatten())
    }
}
