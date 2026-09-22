use std::collections::HashMap;

use beans_engine::Engine;
use beans_lsp_models::OpenDocument;
use lsp_types::{DidCloseTextDocumentParams, DidOpenTextDocumentParams};

pub(crate) fn did_open(
    _engine: &mut Engine,
    open_documents: &mut HashMap<String, OpenDocument>,
    params: DidOpenTextDocumentParams,
) {
    let document = OpenDocument::from(params.text_document);
    open_documents.insert(document.uri.as_str().into(), document);
}

pub(crate) fn did_close(
    _engine: &mut Engine,
    open_documents: &mut HashMap<String, OpenDocument>,
    params: DidCloseTextDocumentParams,
) {
    open_documents.remove(params.text_document.uri.as_str());
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{TextDocumentIdentifier, TextDocumentItem, Uri};

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
