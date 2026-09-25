use std::collections::HashMap;

use crate::open_document::OpenDocument;
use beans_engine::Engine;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

pub(crate) fn did_open(
    engine: &mut Engine,
    open_documents: &mut HashMap<String, OpenDocument>,
    params: DidOpenTextDocumentParams,
) {
    let document = OpenDocument::from(params.text_document);
    engine.process_document(document.uri.as_str(), &document.language_id, &document.text);
    open_documents.insert(document.uri.as_str().into(), document);
}

pub(crate) fn did_change(
    engine: &mut Engine,
    open_documents: &mut HashMap<String, OpenDocument>,
    params: DidChangeTextDocumentParams,
) {
    let Some(document) = open_documents.get_mut(params.text_document.uri.as_str()) else {
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
    engine.process_document(document.uri.as_str(), &document.language_id, &document.text);
}

pub(crate) fn did_close(
    engine: &mut Engine,
    open_documents: &mut HashMap<String, OpenDocument>,
    params: DidCloseTextDocumentParams,
) {
    if open_documents
        .remove(params.text_document.uri.as_str())
        .is_some_and(|document| document.language_id == "java")
    {
        engine.close_document(params.text_document.uri.as_str());
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
        let mut engine = Engine::default();
        let mut documents = HashMap::new();
        did_open(
            &mut engine,
            &mut documents,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem::new(
                    uri.clone(),
                    "java".into(),
                    1,
                    "class C extends Old {}".into(),
                ),
            },
        );
        let change = |version, text: &str| DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier::new(uri.clone(), version),
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.into(),
            }],
        };
        did_change(
            &mut engine,
            &mut documents,
            change(2, "class C extends New {}"),
        );
        did_change(
            &mut engine,
            &mut documents,
            change(1, "class C extends Stale {}"),
        );

        assert_eq!(
            documents.get(uri.as_str()).unwrap().text,
            "class C extends New {}"
        );
        let source = beans_core::model::source::Source::uri(uri.as_str());
        assert_eq!(engine.type_reference_at(&source, 16).unwrap().len(), 3);
    }

    #[test]
    fn opening_and_closing_tracks_the_document() {
        let uri: Uri = "file:///Example.java".parse().unwrap();
        let mut engine = Engine::default();
        let mut documents = HashMap::new();

        did_open(
            &mut engine,
            &mut documents,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem::new(
                    uri.clone(),
                    "java".into(),
                    1,
                    "class Example {}".into(),
                ),
            },
        );
        assert_eq!(
            documents.get(uri.as_str()).unwrap().text,
            "class Example {}"
        );

        did_close(
            &mut engine,
            &mut documents,
            DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier::new(uri.clone()),
            },
        );
        assert!(!documents.contains_key(uri.as_str()));
    }
}
