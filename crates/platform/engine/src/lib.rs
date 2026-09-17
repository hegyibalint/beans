use beans_core_engine::{
    Revision,
    storage::{RevisionEntry, RevisionedStorage},
};
use beans_core_model::source::Source;
use beans_platform_model::Class;

#[derive(Default)]
pub struct PlatformEngine {
    classes: RevisionedStorage<Source, Vec<Class>>,
}

impl PlatformEngine {
    pub fn store(
        &mut self,
        revision: Revision,
        classes: Vec<Class>,
        source: Source,
    ) -> RevisionEntry<Source> {
        self.classes.put(revision, source, classes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_platform_model::{AccessLevel, BinaryName, ClassKind};

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
        let mut engine = PlatformEngine::default();
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
        let mut engine = PlatformEngine::default();
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
        let mut engine = PlatformEngine::default();
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
}
