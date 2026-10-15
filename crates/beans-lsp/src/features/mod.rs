use std::collections::HashMap;

use beans_engine::Engine;
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
    client_capabilities: ClientCapabilities,
    open_documents: HashMap<String, OpenDocument>,
    workspace_documents: HashMap<String, OpenDocument>,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            engine: Engine::new(),
            client_capabilities: ClientCapabilities::default(),
            open_documents: HashMap::new(),
            workspace_documents: HashMap::new(),
        }
    }
}
