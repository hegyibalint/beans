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

        Ok((|| {
            Some(Hover {
                // Bare MarkedString text is Markdown, which can hide generic arguments as HTML tags.
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::PlainText,
                    value: response.contents().into(),
                }),
                range: Some(Range::new(
                    document.position(range.start())?,
                    document.position(range.end())?,
                )),
            })
        })())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{DidOpenTextDocumentParams, TextDocumentItem, Uri};
    use serde_json::json;

    #[test]
    fn hover_response_uses_plaintext_and_utf16_coordinates() {
        let uri: Uri = "untitled:Example.java".parse().unwrap();
        let text = "class Café /*😀*/ extends Box<Café> {}";
        let (_root, mut features) = crate::features::test_features();
        features
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem::new(uri.clone(), "java".into(), 1, text.into()),
            })
            .unwrap();
        let hover_at = |character: u32| {
            let params = serde_json::from_value(json!({
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": character}
            }))
            .unwrap();
            features.hover(&(), &params).unwrap()
        };
        let start = text[..text.find("Box").unwrap()].encode_utf16().count() as u32;
        let end = start + "Box<Café>".encode_utf16().count() as u32;

        assert_eq!(
            serde_json::to_value(hover_at(start)).unwrap(),
            json!({
                "contents": {"kind": "plaintext", "value": "Box<Café>"},
                "range": {"start": {"line": 0, "character": start},
                          "end": {"line": 0, "character": end}}
            })
        );
    }
}
