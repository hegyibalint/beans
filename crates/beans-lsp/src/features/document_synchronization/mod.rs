use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

use super::{Features, source_locations::resource_for_uri};
use crate::model::open_document::OpenDocument;

impl Features {
    pub(crate) fn did_open(&mut self, params: DidOpenTextDocumentParams) {
        let document = OpenDocument::from(params.text_document);
        if let Some(engine) = &mut self.engine {
            if let Err(error) = engine.document_open(
                resource_for_uri(&document.uri),
                &document.language_id,
                &document.text,
            ) {
                log::error!("cannot publish {}: {error}", document.uri.as_str());
                return;
            }
        }
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
        if let Some(engine) = &mut self.engine {
            if let Err(error) = engine.document_edit(
                resource_for_uri(&document.uri),
                &document.language_id,
                &change.text,
            ) {
                log::error!("cannot publish {}: {error}", document.uri.as_str());
                return;
            }
        }
        document.version = params.text_document.version;
        document.text = change.text;
    }

    pub(crate) fn did_close(&mut self, params: DidCloseTextDocumentParams) {
        self.open_documents
            .remove(params.text_document.uri.as_str());
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
        assert_eq!(session.open_documents.get(uri.as_str()).unwrap().version, 2);
    }

    #[test]
    fn untitled_documents_keep_their_language_id_without_a_filename() {
        let uri: Uri = "untitled:Untitled-1".parse().unwrap();
        let mut session = Features::default();
        let open = |language: &str| DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(
                uri.clone(),
                language.into(),
                1,
                "class Example {}".into(),
            ),
        };
        let close = || DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier::new(uri.clone()),
        };

        session.did_open(open("plaintext"));
        assert_eq!(
            session
                .open_documents
                .get(uri.as_str())
                .unwrap()
                .language_id,
            "plaintext"
        );
        session.did_close(close());

        session.did_open(open("java"));
        let document = session.open_documents.get(uri.as_str()).unwrap();
        assert_eq!(document.language_id, "java");
        assert_eq!(document.text, "class Example {}");

        session.did_close(close());
        assert!(!session.open_documents.contains_key(uri.as_str()));
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
