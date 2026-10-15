use beans_core::features::HoverRequest;
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind, Range};

use super::super::{Features, source_locations::resource_for_uri};

impl Features {
    pub(crate) fn hover(&self, params: &HoverParams) -> Result<Option<Hover>, String> {
        let position = &params.text_document_position_params;
        let Some(document) = self.open_documents.get(position.text_document.uri.as_str()) else {
            return Ok(None);
        };
        if document.language_id != "java" {
            return Ok(None);
        }
        let Some(offset) = document.byte_offset(position.position) else {
            return Ok(None);
        };
        let Some(engine) = &self.engine else {
            return Err("Engine is not initialized".into());
        };
        let source = resource_for_uri(&document.uri);
        let Some(response) = engine.feature_hover(&HoverRequest {
            source: &source,
            contents: &document.text,
            offset,
        })?
        else {
            return Ok(None);
        };
        let range = response.range();
        let Some((start, end)) = document
            .position(range.start())
            .zip(document.position(range.end()))
        else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::PlainText,
                value: response.contents().into(),
            }),
            range: Some(Range::new(start, end)),
        }))
    }
}
