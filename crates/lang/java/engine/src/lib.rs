//! Java state management and analysis, independent of LSP.

use beans_core_engine::{
    Revision,
    storage::{RevisionEntry, RevisionedStorage},
};
use beans_core_model::{classpath::Classpath, source::Source};
use beans_lang_java_model::File;
use beans_lang_java_semantics::query::JavaQuery;

#[derive(Default)]
pub struct JavaEngine {
    files: RevisionedStorage<Source, File>,
}

impl JavaEngine {
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

#[cfg(test)]
mod tests {
    use super::*;
    use beans_lang_java_model::names::Name;

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
