use beans_core_engine::{
    Revision,
    storage::{RevisionEntry, RevisionedStorage},
};
use beans_core_model::source::Source;
use beans_platform_jvm_model::classes::Class;
use beans_platform_jvm_semantics::query::JvmQuery;

#[derive(Default)]
pub struct JvmEngine {
    classes: RevisionedStorage<Source, Vec<Class>>,
}

impl JvmEngine {
    pub fn store(
        &mut self,
        revision: Revision,
        classes: Vec<Class>,
        source: Source,
    ) -> RevisionEntry<Source> {
        self.classes.put(revision, source, classes)
    }

    pub fn query(&self, revision: Revision) -> JvmQuery<'_> {
        JvmQuery::new(&self.classes, revision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_platform_jvm_model::{
        classes::{AccessLevel, ClassKind},
        names::BinaryName,
    };

    fn source(path: &str) -> Source {
        Source::SourceFile { path: path.into() }
    }

    fn class(name: &str) -> Class {
        Class::new(
            BinaryName::new(name),
            ClassKind::Class,
            AccessLevel::Package,
        )
    }

    #[test]
    fn a_source_contributes_all_of_its_classes_together() {
        let mut engine = JvmEngine::default();
        let revision = Revision::new(1);
        let source = source("src/example/Outer.java");

        let entry = engine.store(
            revision,
            vec![class("example.Outer"), class("example.Outer$Member")],
            source.clone(),
        );

        assert_eq!(entry.revision, revision);
        assert_eq!(entry.key, source);
        let stored = engine.classes.get(entry.revision, &entry.key).unwrap();
        let names: Vec<_> = stored
            .iter()
            .map(|class| class.binary_name.as_str())
            .collect();
        assert_eq!(names, ["example.Outer", "example.Outer$Member"]);
    }

    #[test]
    fn replacing_a_source_replaces_its_contribution_without_erasing_history() {
        let mut engine = JvmEngine::default();
        let source = source("src/example/Example.java");
        engine.store(
            Revision::new(1),
            vec![class("example.Old"), class("example.Old$Member")],
            source.clone(),
        );
        engine.store(Revision::new(2), vec![class("example.New")], source.clone());

        let old: Vec<_> = engine
            .classes
            .get(Revision::new(1), &source)
            .unwrap()
            .iter()
            .map(|class| class.binary_name.as_str())
            .collect();
        let new: Vec<_> = engine
            .classes
            .get(Revision::new(2), &source)
            .unwrap()
            .iter()
            .map(|class| class.binary_name.as_str())
            .collect();

        assert_eq!(old, ["example.Old", "example.Old$Member"]);
        assert_eq!(new, ["example.New"]);
    }

    #[test]
    fn distinct_sources_can_contribute_the_same_binary_name() {
        let mut engine = JvmEngine::default();
        let revision = Revision::new(1);
        for path in [
            "first/example/Duplicate.java",
            "second/example/Duplicate.java",
        ] {
            engine.store(revision, vec![class("example.Duplicate")], source(path));
        }

        let found = engine
            .classes
            .iter(revision)
            .filter(|(_, classes)| {
                classes
                    .iter()
                    .any(|class| class.binary_name.as_str() == "example.Duplicate")
            })
            .count();

        assert_eq!(found, 2);
    }

    #[test]
    fn queries_read_the_engines_revisioned_classes() {
        let mut engine = JvmEngine::default();
        let revision = Revision::new(1);
        engine.store(
            revision,
            vec![class("example.Example")],
            source("src/example/Example.java"),
        );

        let query = engine.query(revision);
        let found: Vec<_> = query
            .find_class(&BinaryName::new("example.Example"))
            .collect();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].class.binary_name.as_str(), "example.Example");
    }
}
