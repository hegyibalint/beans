use beans_core_engine::{Revision, storage::RevisionedStorage};
use beans_core_model::source::Source;
use beans_platform_jvm_model::{classes::Class, names::BinaryName};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassHandle {
    revision: Revision,
    source: Source,
    index: usize,
}

#[derive(Debug, Clone)]
pub struct ClassEntry<'a> {
    pub handle: ClassHandle,
    pub class: &'a Class,
}

pub struct JvmQuery<'a> {
    classes: &'a RevisionedStorage<Source, Vec<Class>>,
    revision: Revision,
}

impl<'a> JvmQuery<'a> {
    pub fn new(classes: &'a RevisionedStorage<Source, Vec<Class>>, revision: Revision) -> Self {
        Self { classes, revision }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn find_class<'query, 'name>(
        &'query self,
        name: &'name BinaryName,
    ) -> impl Iterator<Item = ClassEntry<'query>> + 'name
    where
        'query: 'name,
    {
        self.classes
            .iter(self.revision)
            .flat_map(move |(source, classes)| {
                classes
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, class)| {
                        (class.binary_name == *name).then(|| ClassEntry {
                            handle: ClassHandle {
                                revision: self.revision,
                                source: source.clone(),
                                index,
                            },
                            class,
                        })
                    })
            })
    }

    /// Reads the exact class selected by a handle, including from an older revision.
    pub fn class<'query>(&'query self, handle: &ClassHandle) -> Option<ClassEntry<'query>> {
        let class = self
            .classes
            .get(handle.revision, &handle.source)?
            .get(handle.index)?;
        Some(ClassEntry {
            handle: handle.clone(),
            class,
        })
    }
}

#[cfg(test)]
mod tests {
    use beans_core_engine::{Revision, storage::RevisionedStorage};
    use beans_core_model::source::Source;
    use beans_platform_jvm_model::{
        classes::{AccessLevel, Class, ClassKind},
        names::BinaryName,
    };

    use super::JvmQuery;

    fn source(path: &str) -> Source {
        Source::ClassFile { path: path.into() }
    }

    fn class(name: &str) -> Class {
        Class::new(
            BinaryName::new(name),
            ClassKind::Class,
            AccessLevel::Package,
        )
    }

    #[test]
    fn an_empty_query_finds_no_classes() {
        let classes = RevisionedStorage::<Source, Vec<Class>>::default();
        let query = JvmQuery::new(&classes, Revision::new(1));

        assert_eq!(query.revision(), Revision::new(1));
        assert_eq!(
            query
                .find_class(&BinaryName::new("example.Missing"))
                .count(),
            0
        );
    }

    #[test]
    fn binary_lookup_is_exact_and_preserves_duplicate_origins() {
        let mut classes = RevisionedStorage::default();
        let revision = Revision::new(1);
        for path in ["first/Example.class", "second/Example.class"] {
            classes.put(
                revision,
                source(path),
                vec![class("example.Example"), class("example.Example$Member")],
            );
        }
        let query = JvmQuery::new(&classes, revision);

        let found: Vec<_> = query
            .find_class(&BinaryName::new("example.Example"))
            .collect();

        assert_eq!(found.len(), 2);
        assert_ne!(found[0].handle, found[1].handle);
        assert!(
            found
                .iter()
                .all(|entry| entry.class.binary_name.as_str() == "example.Example")
        );
        assert_eq!(
            query
                .find_class(&BinaryName::new("example.Example$Missing"))
                .count(),
            0
        );
    }

    #[test]
    fn handles_keep_the_revision_that_selected_the_class() {
        let mut classes = RevisionedStorage::default();
        let origin = source("Example.class");
        classes.put(Revision::new(1), origin.clone(), vec![class("example.Old")]);

        let old_handle = JvmQuery::new(&classes, Revision::new(1))
            .find_class(&BinaryName::new("example.Old"))
            .next()
            .unwrap()
            .handle;

        classes.put(Revision::new(2), origin, vec![class("example.New")]);
        let current = JvmQuery::new(&classes, Revision::new(2));

        assert_eq!(
            current
                .class(&old_handle)
                .unwrap()
                .class
                .binary_name
                .as_str(),
            "example.Old"
        );
        assert_eq!(
            current
                .find_class(&BinaryName::new("example.New"))
                .next()
                .unwrap()
                .class
                .binary_name
                .as_str(),
            "example.New"
        );
    }
}
