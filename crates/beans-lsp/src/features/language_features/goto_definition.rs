use beans_core::model::source::Source;
use lsp_types::{GotoDefinitionParams, Location, Range};

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
        let Source::Source { uri } = target.source else {
            return None;
        };
        let target_document = self
            .open_documents
            .get(&uri)
            .or_else(|| self.workspace_documents.get(&uri))?;
        Some(Location::new(
            uri.parse().ok()?,
            Range::new(
                target_document.position(target.range.start())?,
                target_document.position(target.range.end())?,
            ),
        ))
    }
}
