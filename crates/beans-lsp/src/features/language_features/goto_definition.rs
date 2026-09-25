use beans_core::model::source::Source;
use lsp_types::{GotoDefinitionParams, Location};

use super::super::Features;

impl Features {
    pub(crate) fn goto_definition(&self, params: GotoDefinitionParams) -> Option<Location> {
        let position = params.text_document_position_params;
        let document = self
            .open_documents
            .get(position.text_document.uri.as_str())?;
        let offset = document.byte_offset(position.position)?;
        let target = self
            .engine
            .goto_definition(&Source::uri(document.uri.as_str()), offset)?;
        self.location_for_span(target)
    }
}
