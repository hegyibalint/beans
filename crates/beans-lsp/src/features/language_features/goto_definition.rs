use beans_core::model::source::Source;
use beans_lang::{DefinitionProvider, DefinitionRequest};
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, LocationLink, Range};

use super::super::Features;

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
        let source = Source::uri(document.uri.as_str());
        let classpath = self.workspaces.classpath_for(document.uri.as_str());
        let context = self.languages.feature_context(classpath);
        let definition = self.languages.goto_definition(
            &context,
            &DefinitionRequest {
                source: &source,
                offset,
            },
        )?;
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
    use std::{cell::Cell, rc::Rc};

    use super::*;
    use beans_core::model::classpath::Classpath;
    use beans_lang::HoverProvider;
    use beans_workspace::{Workspace, Workspaces};
    use lsp_types::{DidOpenTextDocumentParams, TextDocumentItem};
    use serde_json::json;

    struct TestWorkspace {
        document: String,
        visible: Source,
        lookups: Rc<Cell<usize>>,
    }

    impl Classpath for TestWorkspace {
        fn contains(&self, source: &Source) -> bool {
            source == &self.visible
        }
    }

    impl Workspace for TestWorkspace {
        fn classpath_for(&self, uri: &str) -> &dyn Classpath {
            assert_eq!(uri, self.document);
            self.lookups.set(self.lookups.get() + 1);
            self
        }
    }

    #[test]
    fn document_queries_obtain_current_workspace_visibility_for_their_uri() {
        // LSP 3.17 #documentUri: preserve the client's URI spelling through dispatch.
        let use_uri = "BEANS-jvm:///library/Use.class";
        let target_uri = "file:///library/Target.java";
        let text = "import library.Target; class Use { Target field; }";
        let lookups = Rc::new(Cell::new(0));
        let workspace = |visible| TestWorkspace {
            document: use_uri.into(),
            visible: Source::uri(visible),
            lookups: lookups.clone(),
        };
        let mut features = Features {
            workspaces: Workspaces::new(workspace(target_uri)),
            ..Features::default()
        };
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
        assert_eq!(lookups.get(), 1);

        assert!(
            features
                .hover(&(), &serde_json::from_value(params.clone()).unwrap())
                .is_some()
        );
        assert_eq!(lookups.get(), 2);

        features.workspaces = Workspaces::new(workspace(use_uri));
        assert!(
            features
                .goto_definition(&(), &serde_json::from_value(params).unwrap())
                .is_none()
        );
        assert_eq!(lookups.get(), 3);
    }
}
