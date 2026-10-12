use beans_core::features::{DefinitionProvider, DefinitionRequest};
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, LocationLink, Range};

use super::super::{Features, source_locations::resource_for_uri};

impl DefinitionProvider<(), GotoDefinitionParams, GotoDefinitionResponse> for Features {
    fn goto_definition(
        &self,
        _context: &(),
        params: &GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        let position = &params.text_document_position_params;
        let document = self
            .open_documents
            .get(position.text_document.uri.as_str())?;
        let offset = document.byte_offset(position.position)?;
        let source = resource_for_uri(&document.uri);
        let definition = self.engine.feature_goto_definition(&DefinitionRequest {
            source: &source,
            offset,
        })?;
        let location = self.location_for_span(definition.target)?;
        // LSP 3.17 #textDocument_definition: links require client support.
        let link_support = self
            .client_capabilities
            .text_document
            .as_ref()
            .and_then(|documents| documents.definition.as_ref())
            .and_then(|definition| definition.link_support)
            .unwrap_or(false);
        if !link_support {
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

#[cfg(test)]
mod tests {
    use super::*;
    use beans_core::features::HoverProvider;
    use lsp_types::{DidOpenTextDocumentParams, TextDocumentItem};
    use serde_json::json;

    #[test]
    fn file_and_virtual_document_queries_use_unrestricted_visibility() {
        for use_uri in ["file:///src/Use.java", "BEANS-jvm:///library/Use.class"] {
            let target_uri = "file:///library/Target.java";
            let text = "import library.Target; class Use { Target field; }";
            let mut features = Features::default();
            for (uri, contents) in [
                (use_uri, text),
                (target_uri, "package library; public class Target {}"),
            ] {
                features.did_open(DidOpenTextDocumentParams {
                    text_document: TextDocumentItem::new(
                        uri.parse().unwrap(),
                        "java".into(),
                        1,
                        contents.into(),
                    ),
                });
            }
            let params = json!({
                "textDocument": {"uri": use_uri},
                "position": {"line": 0, "character": text.rfind("Target").unwrap()}
            });
            let definition = features
                .goto_definition(&(), &serde_json::from_value(params.clone()).unwrap())
                .unwrap();
            let GotoDefinitionResponse::Scalar(location) = definition else {
                panic!("expected a target location");
            };
            assert_eq!(location.uri.as_str(), target_uri);
            assert!(
                features
                    .hover(&(), &serde_json::from_value(params).unwrap())
                    .is_some()
            );
        }
    }
}
