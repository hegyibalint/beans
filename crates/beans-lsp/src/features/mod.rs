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

#[cfg(test)]
pub(crate) fn test_features() -> (tempfile::TempDir, Features) {
    let root = tempfile::tempdir().unwrap();
    let params = lsp_types::InitializeParams {
        workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
            uri: url::Url::from_directory_path(root.path())
                .unwrap()
                .as_str()
                .parse()
                .unwrap(),
            name: "test".into(),
        }]),
        ..lsp_types::InitializeParams::default()
    };
    let features = Features::initialize(&params).unwrap();
    (root, features)
}
