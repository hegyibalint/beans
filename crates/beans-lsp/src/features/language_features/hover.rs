use beans_core::features::{HoverProvider, HoverRequest};
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind, Range};

use super::super::{Features, source_locations::resource_for_uri};

impl HoverProvider<(), HoverParams, Hover> for Features {
    fn hover(&self, _context: &(), params: &HoverParams) -> Result<Option<Hover>, String> {
        let position = &params.text_document_position_params;
        let Some(document) = self.open_documents.get(position.text_document.uri.as_str()) else {
            return Ok(None);
        };
        let Some(offset) = document.byte_offset(position.position) else {
            return Ok(None);
        };
        let source = resource_for_uri(&document.uri);
        let Some(response) = self.engine.feature_hover(&HoverRequest {
            source: &source,
            contents: &document.text,
            offset,
        })?
        else {
            return Ok(None);
        };
        let range = response.range();
        let contents = response.contents();
        let formats = self
            .client_capabilities
            .text_document
            .as_ref()
            .and_then(|document| document.hover.as_ref())
            .and_then(|hover| hover.content_format.as_deref())
            .unwrap_or_default();
        let (kind, value) = formats
            .iter()
            .find_map(|format| match format {
                MarkupKind::Markdown => {
                    contents.markdown().map(|text| (MarkupKind::Markdown, text))
                }
                MarkupKind::PlainText => Some((MarkupKind::PlainText, contents.plaintext())),
            })
            .unwrap_or((MarkupKind::PlainText, contents.plaintext()));

        Ok((|| {
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind,
                    value: value.into(),
                }),
                range: Some(Range::new(
                    document.position(range.start())?,
                    document.position(range.end())?,
                )),
            })
        })())
    }
}
