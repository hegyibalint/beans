use std::collections::HashMap;

use beans_lang::Languages;
use beans_workspace::Workspaces;
use lsp_types::ClientCapabilities;

use crate::model::open_document::OpenDocument;

mod document_synchronization;
mod language_features;
mod source_locations;
mod workspace;

/// Feature handlers compose language state, workspace visibility, and document overlays.
#[derive(Default)]
pub(crate) struct Features {
    languages: Languages,
    workspaces: Workspaces,
    client_capabilities: ClientCapabilities,
    open_documents: HashMap<String, OpenDocument>,
    workspace_documents: HashMap<String, OpenDocument>,
}
