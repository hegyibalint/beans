use beans_core::features::{DefinitionProvider, DefinitionRequest};
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, LocationLink, Range};

use super::super::{Features, source_locations::resource_for_uri};

impl DefinitionProvider<(), GotoDefinitionParams, GotoDefinitionResponse> for Features {
    fn goto_definition(
        &self,
        _context: &(),
        params: &GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>, String> {
        let position = &params.text_document_position_params;
        let Some(document) = self.open_documents.get(position.text_document.uri.as_str()) else {
            return Ok(None);
        };
        let Some(offset) = document.byte_offset(position.position) else {
            return Ok(None);
        };
        let source = resource_for_uri(&document.uri);
        let Some(definition) = self.engine.feature_goto_definition(&DefinitionRequest {
            source: &source,
            offset,
        })?
        else {
            return Ok(None);
        };
        let Some(location) = self.location_for_span(definition.target) else {
            return Ok(None);
        };
        // LSP 3.17 #textDocument_definition: links require client support.
        let link_support = self
            .client_capabilities
            .text_document
            .as_ref()
            .and_then(|documents| documents.definition.as_ref())
            .and_then(|definition| definition.link_support)
            .unwrap_or(false);
        if !link_support {
            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }

        let origin_selection_range = document
            .position(definition.origin_range.start())
            .zip(document.position(definition.origin_range.end()))
            .map(|(start, end)| Range::new(start, end));
        Ok(Some(GotoDefinitionResponse::Link(vec![LocationLink {
            origin_selection_range,
            target_uri: location.uri,
            target_range: location.range,
            target_selection_range: location.range,
        }])))
    }
}
