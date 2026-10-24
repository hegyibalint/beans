use std::collections::HashMap;

use beans_core::event::Notification;
use beans_engine::Engine;
use crossbeam_channel::Receiver;
use lsp_types::ClientCapabilities;

use crate::model::open_document::OpenDocument;

mod diagnostics;
mod document_synchronization;
mod language_features;
mod semantic_tokens;
mod source_locations;
mod workspace;

/// Adapts engine features to LSP capabilities and document overlays.
pub(crate) struct Features {
    engine: Engine,
    pub(crate) engine_notifications: Option<Receiver<Notification>>,
    client_capabilities: ClientCapabilities,
    open_documents: HashMap<String, OpenDocument>,
    workspace_documents: HashMap<String, OpenDocument>,
}
