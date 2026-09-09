//! Java state management and analysis, independent of LSP.

use beans_core_engine::{
    Revision,
    storage::{RevisionEntry, RevisionedStorage},
};
use beans_core_model::source::Source;
use beans_lang_java_model::File;

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

    pub fn analyse(&self, _entry: &RevisionEntry<Source>) {
        todo!("analysis is not implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_lang_java_model::references::NameRef;

    #[test]
    fn store_returns_the_address_of_the_supplied_model() {
        let mut engine = JavaEngine::default();
        let revision = Revision::default();
        let source = Source::SourceFile {
            path: "Example.java".into(),
        };
        let mut model = File::new();
        model.package_name = Some(NameRef::Simple("example".into()));

        let entry = engine.store(revision, model, source.clone());

        assert_eq!(entry.revision, revision);
        assert_eq!(entry.key, source);
        let stored = engine.files.get(entry.revision, &entry.key).unwrap();
        assert_eq!(stored.package_name, Some(NameRef::Simple("example".into())));
    }
}
