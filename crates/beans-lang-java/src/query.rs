use crate::model::{
    File,
    nodes::{NodeIndex, types::TypeDeclaration},
};
use crate::model_query::JavaModelQuery;
pub use crate::semantics::DeclarationHandle;
use beans_core::resolution::TypeDefinitionQuery;
use beans_core::{classpath::Classpath, names::Name, resource::ResourceId};
use beans_core::{query_scope::QueryScope, revision::Revision};

#[derive(Debug, Clone, Copy)]
pub struct JavaTypeEntry<'a> {
    pub source: &'a ResourceId,
    pub file: &'a File,
    pub node_index: NodeIndex,
    pub declaration: &'a TypeDeclaration,
}

pub struct JavaQuery<'a, Q: JavaModelQuery> {
    models: &'a Q,
    scope: QueryScope<'a>,
}

impl<'a, Q: JavaModelQuery> JavaQuery<'a, Q> {
    pub fn new(models: &'a Q, scope: QueryScope<'a>) -> Self {
        Self { models, scope }
    }

    pub fn revision(&self) -> Revision {
        self.scope.revision()
    }

    pub fn classpath(&self) -> &'a dyn Classpath {
        self.scope.classpath()
    }

    pub fn file(&self, source: &ResourceId) -> Option<&'a File> {
        self.models.java_file(source)
    }

    /// The caller supplies a declaration index from this source at the query's revision.
    pub fn declaration_handle(
        &self,
        source: &ResourceId,
        node_index: NodeIndex,
    ) -> DeclarationHandle {
        DeclarationHandle::new(self.scope.revision(), source.clone(), node_index)
    }

    pub fn declaration<'b>(&'b self, handle: &'b DeclarationHandle) -> Option<JavaTypeEntry<'b>> {
        if handle.revision() != self.scope.revision() {
            return None;
        }
        let file = self.models.java_file(handle.source())?;
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

        for (source, file) in self.models.java_files() {
            if !self.scope.classpath().contains_source(source) {
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

impl<Q: JavaModelQuery> TypeDefinitionQuery<DeclarationHandle> for JavaQuery<'_, Q> {
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
    use crate::testing::ModelQuery;
    use beans_core::classpath::Unrestricted;
    use beans_core::resource::ResourceRoot;
    use beans_storage::RevisionedStorage;

    fn name(name: &str) -> Name {
        name.split('.').map(str::to_owned).collect()
    }

    fn source_at(path: &str) -> ResourceId {
        ResourceId::file(std::env::current_dir().unwrap().join(path))
    }

    fn virtual_source(key: &str) -> ResourceId {
        ResourceId::new(ResourceRoot::Virtual {
            provider: "test".into(),
            key: key.into(),
        })
    }

    struct NoSources;

    impl Classpath for NoSources {
        fn contains_source(&self, _source: &ResourceId) -> bool {
            false
        }

        fn contains_class(&self, _source: &ResourceId) -> bool {
            false
        }
    }

    struct SelectedSource<'a>(&'a ResourceId);

    impl Classpath for SelectedSource<'_> {
        fn contains_source(&self, source: &ResourceId) -> bool {
            source == self.0
        }

        fn contains_class(&self, _source: &ResourceId) -> bool {
            false
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
        let models = ModelQuery::new(&files, revision);
        let query = JavaQuery::new(&models, QueryScope::new(revision, &classpath));

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
        let models = ModelQuery::new(&files, revision);
        let query = JavaQuery::new(&models, QueryScope::new(revision, &classpath));
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
    fn discovery_retains_distinct_resources() {
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
        let models = ModelQuery::new(&files, revision);
        let query = JavaQuery::new(&models, QueryScope::new(revision, &classpath));
        let found = query.find_type(&name("p.Example"));

        assert_eq!(found.len(), 3);
        assert_ne!(found[0].source, found[1].source);
        assert_ne!(found[0].source, found[2].source);
        assert_ne!(found[1].source, found[2].source);
        assert!(
            found
                .iter()
                .all(|entry| matches!(entry.source.root, ResourceRoot::File { .. }))
        );
    }

    #[test]
    fn discovery_uses_stored_source_models_not_resource_extensions() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        files.put(
            revision,
            ResourceId::file("src/Example.class"),
            crate::lower_into("package p; class Example {}"),
        );
        let classpath = Unrestricted;
        let models = ModelQuery::new(&files, revision);
        let query = JavaQuery::new(&models, QueryScope::new(revision, &classpath));

        assert_eq!(query.find_type(&name("p.Example")).len(), 1);
    }

    #[test]
    fn discovery_preserves_duplicate_declarations_within_one_resource() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        // Error recovery: duplicate top-level declarations remain distinct.
        files.put(
            revision,
            source_at("src/Example.java"),
            crate::lower_into("package p; class Example {} class Example {}"),
        );
        let classpath = Unrestricted;
        let models = ModelQuery::new(&files, revision);
        let query = JavaQuery::new(&models, QueryScope::new(revision, &classpath));
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
        let old_models = ModelQuery::new(&files, Revision::new(1));
        let deleted_models = ModelQuery::new(&files, Revision::new(3));
        let old = JavaQuery::new(&old_models, QueryScope::new(Revision::new(1), &classpath));
        let deleted = JavaQuery::new(
            &deleted_models,
            QueryScope::new(Revision::new(3), &classpath),
        );

        assert_eq!(old.find_type(&name("p.Example")).len(), 1);
        assert!(deleted.find_type(&name("p.Example")).is_empty());
        assert!(deleted.find_type(&name("p.Other")).is_empty());
    }

    #[test]
    fn unrestricted_classpath_sees_all_stored_source_models() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        for source in [
            source_at("src/Target.java"),
            source_at("elsewhere/Target.java"),
            virtual_source("untitled:Target.java"),
            ResourceId::file("src/Target.class"),
        ] {
            files.put(
                revision,
                source,
                crate::lower_into("package p; class Target {}"),
            );
        }

        let models = ModelQuery::new(&files, revision);
        let all = Unrestricted;
        let unrestricted = JavaQuery::new(&models, QueryScope::new(revision, &all));
        let empty = NoSources;
        let excluded = JavaQuery::new(&models, QueryScope::new(revision, &empty));

        assert_eq!(unrestricted.find_type(&name("p.Target")).len(), 4);
        assert!(excluded.find_type(&name("p.Target")).is_empty());
    }

    #[test]
    fn discovery_uses_custom_classpath_visibility_for_virtual_sources() {
        let mut files = RevisionedStorage::default();
        let revision = Revision::new(1);
        let selected = virtual_source("untitled:Target.java");
        for source in [source_at("src/Target.java"), selected.clone()] {
            files.put(
                revision,
                source,
                crate::lower_into("package p; class Target {}"),
            );
        }
        let classpath = SelectedSource(&selected);
        let models = ModelQuery::new(&files, revision);
        let query = JavaQuery::new(&models, QueryScope::new(revision, &classpath));

        let found = query.find_type(&name("p.Target"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].source, &selected);
    }

    #[test]
    fn declaration_reads_reject_handles_from_a_different_view() {
        let mut files = RevisionedStorage::default();
        let source = source_at("Example.java");
        let revision = Revision::new(2);
        files.put(
            revision,
            source.clone(),
            crate::lower_into("class Example {}"),
        );
        let models = ModelQuery::new(&files, revision);
        let query = JavaQuery::new(&models, QueryScope::new(revision, &Unrestricted));
        let found = query.find_type(&name("Example"));
        let current = query.declaration_handle(&source, found[0].node_index);
        let older = DeclarationHandle::new(Revision::new(1), source, current.node_index());

        assert!(query.declaration(&current).is_some());
        assert!(query.declaration(&older).is_none());
    }

    #[test]
    fn context_retains_the_supplied_revision_and_classpath() {
        let files = RevisionedStorage::default();
        let revision = Revision::new(7);
        let source = virtual_source("untitled:Example.java");
        let classpath = SelectedSource(&source);
        let models = ModelQuery::new(&files, revision);
        let query = JavaQuery::new(&models, QueryScope::new(revision, &classpath));

        assert_eq!(query.revision(), revision);
        assert!(std::ptr::eq(
            query.classpath(),
            &classpath as &dyn Classpath
        ));
    }
}
