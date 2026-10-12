use std::path::Path;

use beans_core::resource::{ResourceId, ResourceRoot};

use crate::Engine;

#[cfg(test)]
mod tests;

impl Engine {
    pub fn accept(&self, path: &Path) -> bool {
        self.java.accept(path)
    }

    pub fn process(&mut self, source: ResourceId, contents: &str) {
        let ResourceRoot::File { path } = &source.root else {
            return;
        };
        if !source.entries.is_empty() || !self.accept(path) {
            return;
        }
        self.process_document(source, "java", contents);
    }

    pub fn process_document(&mut self, source: ResourceId, language_id: &str, contents: &str) {
        if language_id != "java" {
            return;
        }
        let model = self.java.process(contents);
        let revision = self.revision.advance();
        self.java.store(revision, model, source);
    }

    pub fn close_document(&mut self, source: ResourceId) {
        let revision = self.revision.advance();
        self.java.remove(revision, source);
    }
}
