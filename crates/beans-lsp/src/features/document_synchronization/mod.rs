use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

use super::Features;
use crate::model::open_document::OpenDocument;

impl Features {
    pub(crate) fn did_open(&mut self, params: DidOpenTextDocumentParams) {
        let document = OpenDocument::from(params.text_document);
        self.engine
            .process_document(document.uri.as_str(), &document.language_id, &document.text);
        self.open_documents
            .insert(document.uri.as_str().into(), document);
    }

    pub(crate) fn did_change(&mut self, params: DidChangeTextDocumentParams) {
        let Some(document) = self
            .open_documents
            .get_mut(params.text_document.uri.as_str())
        else {
            return;
        };
        if params.text_document.version <= document.version || params.content_changes.len() != 1 {
            return;
        }
        let change = params.content_changes.into_iter().next().unwrap();
        if change.range.is_some() {
            return;
        }
        document.version = params.text_document.version;
        document.text = change.text;
        self.engine
            .process_document(document.uri.as_str(), &document.language_id, &document.text);
    }

    pub(crate) fn did_close(&mut self, params: DidCloseTextDocumentParams) {
        if self
            .open_documents
            .remove(params.text_document.uri.as_str())
            .is_some_and(|document| document.language_id == "java")
        {
            if let Some(indexed) = self
                .workspace_documents
                .get(params.text_document.uri.as_str())
            {
                self.engine.process_document(
                    indexed.uri.as_str(),
                    &indexed.language_id,
                    &indexed.text,
                );
            } else {
                self.engine
                    .close_document(params.text_document.uri.as_str());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{
        TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, Uri,
        VersionedTextDocumentIdentifier,
    };

    #[test]
    fn full_changes_replace_the_document_and_stale_versions_do_not() {
        let uri: Uri = "untitled:Example.java".parse().unwrap();
        let mut session = Features::default();
        session.did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(
                uri.clone(),
                "java".into(),
                1,
                "class C extends Old {}".into(),
            ),
        });
        let change = |version, text: &str| DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier::new(uri.clone(), version),
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.into(),
            }],
        };
        session.did_change(change(2, "class C extends New {}"));
        session.did_change(change(1, "class C extends Stale {}"));

        assert_eq!(
            session.open_documents.get(uri.as_str()).unwrap().text,
            "class C extends New {}"
        );
        let source = beans_core::model::source::Source::uri(uri.as_str());
        assert_eq!(
            session.engine.type_reference_at(&source, 16).unwrap().len(),
            3
        );
    }

    #[test]
    fn opening_and_closing_tracks_the_document() {
        let uri: Uri = "file:///Example.java".parse().unwrap();
        let mut session = Features::default();

        session.did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(
                uri.clone(),
                "java".into(),
                1,
                "class Example {}".into(),
            ),
        });
        assert_eq!(
            session.open_documents.get(uri.as_str()).unwrap().text,
            "class Example {}"
        );

        session.did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier::new(uri.clone()),
        });
        assert!(!session.open_documents.contains_key(uri.as_str()));
    }
}
