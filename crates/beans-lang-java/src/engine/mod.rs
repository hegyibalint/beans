//! Java state management and analysis, independent of LSP.

use std::path::Path;

use self::query::JavaQuery;
use crate::{lowering::lower_into, model::File};
use beans_core::location::Location;
use beans_core::{query_scope::QueryScope, revision::Revision};
use beans_storage::{RevisionEntry, RevisionedStorage};

pub mod query;

#[derive(Default)]
pub struct JavaLanguage {
    files: RevisionedStorage<Location, File>,
}

impl JavaLanguage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept(&self, path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == "java")
    }

    pub fn process(&self, contents: &str) -> File {
        lower_into(contents)
    }

    /// Reads a stored Java file at the requested revision.
    pub fn file(&self, revision: Revision, source: &Location) -> Option<&File> {
        self.files.get(revision, source)
    }

    pub fn remove(&mut self, revision: Revision, source: Location) {
        self.files.remove(revision, source);
    }

    pub fn store(
        &mut self,
        revision: Revision,
        model: File,
        source: Location,
    ) -> RevisionEntry<Location> {
        self.files.put(revision, source, model)
    }

    pub fn query<'a>(&'a self, scope: QueryScope<'a>) -> JavaQuery<'a> {
        JavaQuery::new(&self.files, scope)
    }

    pub fn analyse(&self, _entry: &RevisionEntry<Location>, _scope: QueryScope<'_>) {
        todo!("analysis is not implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_core::names::Name;

    #[test]
    fn accept_recognizes_java_source_paths() {
        let engine = JavaLanguage::default();

        assert!(engine.accept(Path::new("src/example/Example.java")));
        assert!(engine.accept(Path::new("virtual/*Example.java")));
        assert!(!engine.accept(Path::new("README.md")));
        assert!(!engine.accept(Path::new("Example.class")));
    }

    #[test]
    fn process_lowers_java_source_without_storing_it() {
        let engine = JavaLanguage::default();

        let file = engine.process("package example; class Example {}");

        assert_eq!(file.package_name.as_slice(), ["example"]);
    }

    #[test]
    fn store_returns_the_address_of_the_supplied_model() {
        let mut engine = JavaLanguage::default();
        let revision = Revision::default();
        let source = Location::file("/Example.java");
        let mut model = File::new();
        model.package_name = Name::new(vec!["example".into()]);

        let entry = engine.store(revision, model, source.clone());

        assert_eq!(entry.revision, revision);
        assert_eq!(entry.key, source);
        let stored = engine.files.get(entry.revision, &entry.key).unwrap();
        assert_eq!(stored.package_name.as_slice(), ["example"]);
    }
}
