use std::collections::HashMap;

use beans_core::resource::{ResourceId, ResourceRoot};

use crate::Engine;

pub struct Documents {
    entries: HashMap<ResourceId, Entry>,
}

impl Documents {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, source: &ResourceId) -> Option<&Entry> {
        self.entries.get(source)
    }

    pub fn insert(&mut self, source: ResourceId, entry: Entry) -> Option<Entry> {
        self.entries.insert(source, entry)
    }

    pub fn remove(&mut self, source: &ResourceId) -> Option<Entry> {
        self.entries.remove(source)
    }
}

pub struct Entry {
    pub model: Model,
    pub content: Option<Content>,
}

pub enum Model {
    Java(beans_lang_java::model::File),
    Jvm(beans_platform_jvm::Class),
}

pub enum Content {
    Stored(Vec<u8>),
}

impl Engine {
    pub fn document_open(&mut self, source: ResourceId, language_id: &str, contents: &str) {
        self.document_process(source, language_id, contents);
    }

    pub fn document_edit(&mut self, source: ResourceId, language_id: &str, contents: &str) {
        self.document_process(source, language_id, contents);
    }

    pub fn document_close(&mut self, source: ResourceId) {
        let revision = self.revision.advance();
        self.java.remove(revision, source);
    }

    pub fn process(&mut self, source: ResourceId, contents: &str) {
        let ResourceRoot::File { path } = &source.root else {
            return;
        };
        if !source.entries.is_empty() || !self.java.accept(path) {
            return;
        }
        self.document_process(source, "java", contents);
    }

    fn document_process(&mut self, source: ResourceId, language_id: &str, contents: &str) {
        if language_id != "java" {
            return;
        }
        let model = self.java.process(contents);
        let revision = self.revision.advance();
        self.java.store(revision, model, source);
    }
}

#[cfg(test)]
mod tests;
