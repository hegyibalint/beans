use std::collections::HashMap;

use beans_engine::Engine;
use lsp_types::ClientCapabilities;

use crate::model::open_document::OpenDocument;

mod document_synchronization;
mod language_features;
mod source_locations;
mod workspace;
pub(crate) use workspace::InitializeError;

/// Adapts LSP capabilities, documents, and requests to Engine.
pub(crate) struct Features {
    engine: Option<Engine>,
    client_capabilities: ClientCapabilities,
    open_documents: HashMap<String, OpenDocument>,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            engine: None,
            client_capabilities: ClientCapabilities::default(),
            open_documents: HashMap::new(),
        }
    }
}
