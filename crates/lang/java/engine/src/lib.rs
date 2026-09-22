//! Java state management and analysis, independent of LSP.

use std::path::Path;

use beans_core_engine::{
    Revision,
    storage::{RevisionEntry, RevisionedStorage},
};
use beans_core_model::{
    classpath::Classpath,
    names::Name,
    source::{Source, SourceLocation},
};
use beans_core_resolution::query::TypeDefinitionQuery;
use beans_lang_java_model::File;
use beans_lang_java_semantics::{
    lower_into,
    query::{DeclarationHandle, JavaQuery},
};

#[derive(Default)]
pub struct JavaEngine {
    files: RevisionedStorage<Source, File>,
}

impl JavaEngine {
    pub fn accept(&self, path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == "java")
    }

    pub fn process(&self, contents: &str) -> File {
        lower_into(contents)
    }

    pub fn find_declaration(
        &self,
        _revision: Revision,
        _source: &Source,
        _offset: usize,
    ) -> Option<SourceLocation> {
        None
    }

    pub fn store(
        &mut self,
        revision: Revision,
        model: File,
        source: Source,
    ) -> RevisionEntry<Source> {
        self.files.put(revision, source, model)
    }

    pub fn query<'a>(&'a self, revision: Revision, classpath: &'a Classpath) -> JavaQuery<'a> {
        JavaQuery::new(&self.files, revision, classpath)
    }

    pub fn analyse(&self, _entry: &RevisionEntry<Source>, _classpath: &Classpath) {
        todo!("analysis is not implemented")
    }
}

impl TypeDefinitionQuery<DeclarationHandle> for JavaEngine {
    fn find_types<'query>(
        &'query self,
        _name: &'query Name,
    ) -> impl Iterator<Item = DeclarationHandle> + 'query {
        Vec::new().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_core_model::names::Name;

    #[test]
    fn accept_recognizes_java_source_paths() {
        let engine = JavaEngine::default();

        assert!(engine.accept(Path::new("src/example/Example.java")));
        assert!(engine.accept(Path::new("virtual/*Example.java")));
        assert!(!engine.accept(Path::new("README.md")));
        assert!(!engine.accept(Path::new("Example.class")));
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
        let source = Source::SourceFile {
            path: "Example.java".into(),
        };
        let mut model = File::new();
        model.package_name = Name::new(vec!["example".into()]);

        let entry = engine.store(revision, model, source.clone());

        assert_eq!(entry.revision, revision);
        assert_eq!(entry.key, source);
        let stored = engine.files.get(entry.revision, &entry.key).unwrap();
        assert_eq!(stored.package_name.as_slice(), ["example"]);
    }
}
