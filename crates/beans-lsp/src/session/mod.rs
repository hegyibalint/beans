use std::collections::HashMap;

use beans_engine::Engine;

use crate::open_document::OpenDocument;

mod hover;
mod text_document;

/// The engine and the client's open-document overlays, kept together.
#[derive(Default)]
pub(super) struct Session {
    engine: Engine,
    open_documents: HashMap<String, OpenDocument>,
}
