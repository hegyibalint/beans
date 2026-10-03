//! Java state management and analysis, independent of LSP.

use std::path::Path;

use self::query::JavaQuery;
use crate::{lowering::lower_into, model::File};
use beans_core::origin::Origin;
use beans_core::{query_scope::QueryScope, revision::Revision};
use beans_storage::{RevisionEntry, RevisionedStorage};
use url::Url;

pub mod query;

#[derive(Default)]
pub struct JavaEngine {
    files: RevisionedStorage<Origin, File>,
}

impl JavaEngine {
    pub fn accept(&self, path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == "java")
    }

    pub fn accept_uri(&self, uri: &str) -> bool {
        let Ok(url) = Url::parse(uri) else {
            return false;
        };
        url.scheme() == "file"
            && url
                .to_file_path()
                .ok()
                .is_some_and(|path| self.accept(&path))
    }

    pub fn process(&self, contents: &str) -> File {
        lower_into(contents)
    }

    /// Reads a stored Java file at the requested revision.
    pub fn file(&self, revision: Revision, source: &Origin) -> Option<&File> {
        self.files.get(revision, source)
    }

    pub fn remove(&mut self, revision: Revision, source: Origin) {
        self.files.remove(revision, source);
    }

    pub fn store(
        &mut self,
        revision: Revision,
        model: File,
        source: Origin,
    ) -> RevisionEntry<Origin> {
        self.files.put(revision, source, model)
    }

    pub fn query<'a>(&'a self, scope: QueryScope<'a>) -> JavaQuery<'a> {
        JavaQuery::new(&self.files, scope)
    }

    pub fn analyse(&self, _entry: &RevisionEntry<Origin>, _scope: QueryScope<'_>) {
        todo!("analysis is not implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_core::names::Name;

    #[test]
    fn accept_recognizes_java_source_paths() {
        let engine = JavaEngine::default();

        assert!(engine.accept(Path::new("src/example/Example.java")));
        assert!(engine.accept(Path::new("virtual/*Example.java")));
        assert!(!engine.accept(Path::new("README.md")));
        assert!(!engine.accept(Path::new("Example.class")));
        assert!(engine.accept_uri("file:///src/example/Example.java"));
        assert!(!engine.accept_uri("untitled:Example.java"));
        assert!(!engine.accept_uri("file:///src/example/Example.class"));
    }

    #[test]
    fn process_lowers_java_source_without_storing_it() {
        let engine = JavaEngine::default();

        let file = engine.process("package example; class Example {}");

        assert_eq!(file.package_name.as_slice(), ["example"]);
    }

    #[test]
    fn store_returns_the_address_of_the_supplied_model() {
        let mut engine = JavaEngine::default();
        let revision = Revision::default();
        let source = Origin::uri("file:///Example.java");
        let mut model = File::new();
        model.package_name = Name::new(vec!["example".into()]);

        let entry = engine.store(revision, model, source.clone());

        assert_eq!(entry.revision, revision);
        assert_eq!(entry.key, source);
        let stored = engine.files.get(entry.revision, &entry.key).unwrap();
        assert_eq!(stored.package_name.as_slice(), ["example"]);
    }
}
