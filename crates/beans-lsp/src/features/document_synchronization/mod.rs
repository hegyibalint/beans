use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
};

use super::{Features, source_locations::resource_for_uri};
use crate::model::open_document::OpenDocument;

impl Features {
    pub(crate) fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Result<bool, String> {
        let document = OpenDocument::from(params.text_document);
        let changed = self.engine.process_document(
            resource_for_uri(&document.uri),
            &document.language_id,
            &document.text,
        )?;
        self.open_documents
            .insert(document.uri.as_str().into(), document);
        Ok(changed)
    }

    pub(crate) fn did_change(
        &mut self,
        params: DidChangeTextDocumentParams,
    ) -> Result<bool, String> {
        let Some(document) = self
            .open_documents
            .get_mut(params.text_document.uri.as_str())
        else {
            return Ok(false);
        };
        if params.text_document.version <= document.version || params.content_changes.len() != 1 {
            return Ok(false);
        }
        let change = params.content_changes.into_iter().next().unwrap();
        if change.range.is_some() {
            return Ok(false);
        }
        let changed = self.engine.process_document(
            resource_for_uri(&document.uri),
            &document.language_id,
            &change.text,
        )?;
        document.version = params.text_document.version;
        document.text = change.text;
        Ok(changed)
    }

    pub(crate) fn did_close(&mut self, params: DidCloseTextDocumentParams) -> Result<bool, String> {
        let changed = if self
            .open_documents
            .get(params.text_document.uri.as_str())
            .is_some_and(|document| document.language_id == "java")
        {
            if let Some(indexed) = self
                .workspace_documents
                .get(params.text_document.uri.as_str())
            {
                self.engine.process_document(
                    resource_for_uri(&indexed.uri),
                    &indexed.language_id,
                    &indexed.text,
                )?
            } else {
                self.engine
                    .close_document(resource_for_uri(&params.text_document.uri))?
            }
        } else {
            false
        };
        self.open_documents
            .remove(params.text_document.uri.as_str());
        Ok(changed)
    }
}
