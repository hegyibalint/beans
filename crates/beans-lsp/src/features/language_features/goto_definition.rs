use beans_core::model::source::Source;
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, LocationLink, Range};

use super::super::Features;

impl Features {
    pub(crate) fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        let position = params.text_document_position_params;
        let document = self
            .open_documents
            .get(position.text_document.uri.as_str())?;
        let offset = document.byte_offset(position.position)?;
        let source = Source::uri(document.uri.as_str());
        let definition = self
            .engine
            .language_features()
            .goto_definition(&source, offset)?;
        let location = self.location_for_span(definition.target)?;
        if !self.definition_link_support {
            return Some(GotoDefinitionResponse::Scalar(location));
        }

        let origin_selection_range = document
            .position(definition.origin_range.start())
            .zip(document.position(definition.origin_range.end()))
            .map(|(start, end)| Range::new(start, end));
        Some(GotoDefinitionResponse::Link(vec![LocationLink {
            origin_selection_range,
            target_uri: location.uri,
            target_range: location.range,
            target_selection_range: location.range,
        }]))
    }
}
