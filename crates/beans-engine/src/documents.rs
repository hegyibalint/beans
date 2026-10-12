use beans_core::resource::{ResourceId, ResourceRoot};

use crate::{
    Engine,
    storage::{Content, Entry, Model},
};

impl Engine {
    pub fn document_open(&mut self, source: ResourceId, language_id: &str, contents: &str) {
        self.document_process(source, language_id, contents);
    }

    pub fn document_edit(&mut self, source: ResourceId, language_id: &str, contents: &str) {
        self.document_process(source, language_id, contents);
    }

    pub fn document_close(&mut self, source: ResourceId) {
        self.revision.advance();
        self.storage.remove(&source);
    }

    pub fn process(&mut self, source: ResourceId, contents: &str) {
        let ResourceRoot::File { path } = &source.root else {
            return;
        };
        if !source.entries.is_empty() || !beans_lang_java::accept(path) {
            return;
        }
        self.document_process(source, "java", contents);
    }

    fn document_process(&mut self, source: ResourceId, language_id: &str, contents: &str) {
        if language_id != "java" {
            return;
        }
        let Some(model) = beans_lang_java::process(&source, contents) else {
            return;
        };
        self.revision.advance();
        self.storage.insert(
            source,
            Entry {
                model: Model::Java(model),
                content: Some(Content::Stored(contents.as_bytes().to_vec())),
            },
        );
    }
}

#[cfg(test)]
mod tests;
