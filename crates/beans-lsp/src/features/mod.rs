use std::collections::HashMap;

use beans_engine::Engine;

use crate::model::open_document::OpenDocument;

mod document_synchronization;
mod language_features;

/// Feature handlers share an engine and the client's open-document overlays.
#[derive(Default)]
pub(crate) struct Features {
    engine: Engine,
    open_documents: HashMap<String, OpenDocument>,
}
