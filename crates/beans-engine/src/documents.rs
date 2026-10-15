use std::path::Path;

use beans_core::location::{Location, LocationEntry};

use crate::Engine;

#[cfg(test)]
mod tests;

impl Engine {
    pub fn accept(&self, path: &Path) -> bool {
        self.java.accept(path)
    }

    pub fn process(&mut self, source: Location, contents: &str) {
        let [LocationEntry::File(path)] = source.entries.as_slice() else {
            return;
        };
        if !self.accept(path) {
            return;
        }
        self.process_document(source, "java", contents);
    }

    pub fn process_document(&mut self, source: Location, language_id: &str, contents: &str) {
        if language_id != "java" {
            return;
        }
        let model = self.java.process(contents);
        let revision = self.revision.advance();
        self.java.store(revision, model, source);
    }

    pub fn close_document(&mut self, source: Location) {
        let revision = self.revision.advance();
        self.java.remove(revision, source);
    }
}
