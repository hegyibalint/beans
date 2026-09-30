use crate::model::{
    File,
    nodes::{NodeIndex, types::TypeDeclaration},
};
pub use crate::semantics::DeclarationHandle;
use beans_core::engine::{QueryEnvironment, Revision, storage::RevisionedStorage};
use beans_core::model::{classpath::Classpath, names::Name, source::Source};
use beans_core::resolution::query::TypeDefinitionQuery;

#[derive(Debug, Clone, Copy)]
pub struct JavaTypeEntry<'a> {
    pub source: &'a Source,
    pub file: &'a File,
    pub node_index: NodeIndex,
    pub declaration: &'a TypeDeclaration,
}

pub struct JavaQuery<'a> {
    files: &'a RevisionedStorage<Source, File>,
    environment: QueryEnvironment<'a>,
}

impl<'a> JavaQuery<'a> {
    pub fn new(
        files: &'a RevisionedStorage<Source, File>,
        environment: QueryEnvironment<'a>,
    ) -> Self {
        Self { files, environment }
    }

    pub fn revision(&self) -> Revision {
        self.environment.revision()
    }

    pub fn classpath(&self) -> &'a dyn Classpath {
        self.environment.classpath()
    }

    /// Reads a stored model at this revision, without applying classpath visibility.
    pub fn file(&self, source: &Source) -> Option<&'a File> {
        self.files.get(self.environment.revision(), source)
    }

    /// The caller supplies a declaration index from this source at the query's revision.
    pub fn declaration_handle(&self, source: &Source, node_index: NodeIndex) -> DeclarationHandle {
        DeclarationHandle::new(self.environment.revision(), source.clone(), node_index)
    }

    /// Reads a type at the handle's revision, without applying classpath visibility.
    pub fn declaration<'b>(&'b self, handle: &'b DeclarationHandle) -> Option<JavaTypeEntry<'b>> {
        let file = self.files.get(handle.revision(), handle.source())?;
        let declaration = file.node(handle.node_index())?.kind().as_type()?;
        Some(JavaTypeEntry {
            source: handle.source(),
            file,
            node_index: handle.node_index(),
            declaration,
        })
    }

    /// Finds declarations by canonical name (JLS §6.7) in stored source files.
    /// Uses the supplied classpath for visibility. Does not check Java
    /// accessibility or search inherited members.
    pub fn find_type(&self, name: &Name) -> Vec<JavaTypeEntry<'a>> {
        let mut candidates = Vec::new();

        for (source, file) in self.files.iter(self.environment.revision()) {
            if !matches!(source, Source::Source { .. })
                || !self.environment.classpath().contains(source)
            {
                continue;
            }

            for (node_index, declaration) in file.find_type(name) {
                candidates.push(JavaTypeEntry {
                    source,
                    file,
                    node_index,
                    declaration,
                });
            }
        }
        candidates
    }
}

impl TypeDefinitionQuery<DeclarationHandle> for JavaQuery<'_> {
    fn find_types<'query>(
        &'query self,
        name: &'query Name,
    ) -> impl Iterator<Item = DeclarationHandle> + 'query {
        self.find_type(name)
            .into_iter()
            .map(|entry| self.declaration_handle(entry.source, entry.node_index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_core::model::classpath::Unrestricted;
    use url::Url;

    fn name(name: &str) -> Name {
        name.split('.').map(str::to_owned).collect()
    }

    fn source_at(path: &str) -> Source {
        Source::uri(
            Url::from_file_path(std::env::current_dir().unwrap().join(path))
                .unwrap()
                .to_string(),
        )
    }

    struct NoSources;

    impl Classpath for NoSources {
        fn contains(&self, _source: &Source) -> bool {
            false
        }
    }

    struct SelectedSource<'a>(&'a Source);

    impl Classpath for SelectedSource<'_> {
        fn contains(&self, source: &Source) -> bool {
            source == self.0
        }
    }

    #[test]
    fn file_reads_use_the_bound_revision_and_source_without_cloning() {
        let mut files = RevisionedStorage::default();
        let source = source_at("Example.java");
        let other = source_at("Other.java");
        let revision = Revision::new(2);
        files.put(revision, source.clone(), File::new());
        files.put(revision, other.clone(), File::new());
        files.put(Revision::new(5), source.clone(), File::new());
        let classpath = NoSources;
        let query = JavaQuery::new(&files, QueryEnvironment::new(revision, &classpath));

        let expected = files.get(revision, &source).unwrap();
        let actual = query.file(&source).unwrap();
        assert!(std::ptr::eq(actual, expected));
        assert!(!std::ptr::eq(actual, query.file(&other).unwrap()));
        assert!(!std::ptr::eq(
            actual,
            files.get(Revision::new(5), &source).unwrap()
        ));
    }

    #[test]
    fn discovery_delegates_canonical_member_lookup_to_visible_files() {
        let mut files = RevisionedStorage::default();
        let source = source_at("src/misplaced/Example.java");
        let revision = Revision::new(1);
        files.put(
            revision,
            source.clone(),
            crate::lower_into("package p.q; class Outer { private class Member {} }"),
        );
        let classpath = Unrestricted;
        let query = JavaQuery::new(&files, QueryEnvironment::new(revision, &classpath));
        let found = query.find_type(&name("p.q.Outer.Member"));

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].source, &source);
        let stored = found[0]
            .file
            .node(found[0].node_index)
            .unwrap()
            .kind()
            .as_type()
            .expect("expected a type declaration");
        assert!(std::ptr::eq(found[0].declaration, stored));
        assert_eq!(stored.name(), Some("Member"));
    }

    #[test]
    fn discovery_retains_distinct_origins() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        for path in [
            "src/a/Example.java",
            "src/b/Example.java",
            "other/Example.java",
        ] {
            files.put(
                revision,
                source_at(path),
                crate::lower_into("package p; class Example {}"),
            );
        }
        let classpath = Unrestricted;
        let query = JavaQuery::new(&files, QueryEnvironment::new(revision, &classpath));
        let found = query.find_type(&name("p.Example"));

        assert_eq!(found.len(), 3);
        assert_ne!(found[0].source, found[1].source);
        assert_ne!(found[0].source, found[2].source);
        assert_ne!(found[1].source, found[2].source);
        assert!(
            found
                .iter()
                .all(|entry| matches!(entry.source, Source::Source { .. }))
        );
    }

    #[test]
    fn discovery_excludes_non_source_origins() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        files.put(
            revision,
            Source::class_file("src/Example.class"),
            crate::lower_into("package p; class Example {}"),
        );
        let classpath = Unrestricted;
        let query = JavaQuery::new(&files, QueryEnvironment::new(revision, &classpath));

        assert!(query.find_type(&name("p.Example")).is_empty());
    }

    #[test]
    fn discovery_preserves_duplicate_declarations_within_one_origin() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        // Error recovery: duplicate top-level declarations remain distinct.
        files.put(
            revision,
            source_at("src/Example.java"),
            crate::lower_into("package p; class Example {} class Example {}"),
        );
        let classpath = Unrestricted;
        let query = JavaQuery::new(&files, QueryEnvironment::new(revision, &classpath));
        let found = query.find_type(&name("p.Example"));

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].source, found[1].source);
        assert_ne!(found[0].node_index, found[1].node_index);
    }

    #[test]
    fn discovery_uses_the_bound_revision_after_replacement_and_deletion() {
        let mut files = RevisionedStorage::default();
        let source = source_at("src/Example.java");
        files.put(
            Revision::new(1),
            source.clone(),
            crate::lower_into("package p; class Example {}"),
        );
        files.put(
            Revision::new(2),
            source.clone(),
            crate::lower_into("package p; class Other {}"),
        );
        files.remove(Revision::new(3), source);
        let classpath = Unrestricted;
        let old = JavaQuery::new(&files, QueryEnvironment::new(Revision::new(1), &classpath));
        let deleted = JavaQuery::new(&files, QueryEnvironment::new(Revision::new(3), &classpath));

        assert_eq!(old.find_type(&name("p.Example")).len(), 1);
        assert!(deleted.find_type(&name("p.Example")).is_empty());
        assert!(deleted.find_type(&name("p.Other")).is_empty());
    }

    #[test]
    fn unrestricted_classpath_sees_all_stored_source_origins() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        for source in [
            source_at("src/Target.java"),
            source_at("elsewhere/Target.java"),
            Source::uri("untitled:Target.java"),
            Source::class_file("src/Target.class"),
        ] {
            files.put(
                revision,
                source,
                crate::lower_into("package p; class Target {}"),
            );
        }

        let all = Unrestricted;
        let unrestricted = JavaQuery::new(&files, QueryEnvironment::new(revision, &all));
        let empty = NoSources;
        let excluded = JavaQuery::new(&files, QueryEnvironment::new(revision, &empty));

        assert_eq!(unrestricted.find_type(&name("p.Target")).len(), 3);
        assert!(excluded.find_type(&name("p.Target")).is_empty());
    }

    #[test]
    fn discovery_uses_custom_classpath_visibility_for_virtual_sources() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        let selected = Source::uri("untitled:Target.java");
        for source in [source_at("src/Target.java"), selected.clone()] {
            files.put(
                revision,
                source,
                crate::lower_into("package p; class Target {}"),
            );
        }
        let classpath = SelectedSource(&selected);
        let query = JavaQuery::new(&files, QueryEnvironment::new(revision, &classpath));

        let found = query.find_type(&name("p.Target"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].source, &selected);
    }

    #[test]
    fn context_retains_the_supplied_revision_and_classpath() {
        let files = RevisionedStorage::default();
        let revision = Revision::new(7);
        let source = Source::uri("untitled:Example.java");
        let classpath = SelectedSource(&source);
        let query = JavaQuery::new(&files, QueryEnvironment::new(revision, &classpath));

        assert_eq!(query.revision(), revision);
        assert!(std::ptr::eq(
            query.classpath(),
            &classpath as &dyn Classpath
        ));
    }
}
