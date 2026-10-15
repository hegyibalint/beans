use super::*;
use crate::testing::ModelQuery;
use beans_core::{classpath::Unrestricted, resource::ResourceRoot};
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
fn file_reads_return_owned_models_from_the_bound_revision() {
    let source = source_at("Example.java");
    let actual = {
        let mut files = RevisionedStorage::default();
        files.put(
            Revision::new(2),
            source.clone(),
            crate::lower_into("class Former {}"),
        );
        files.put(
            Revision::new(5),
            source.clone(),
            crate::lower_into("class Latest {}"),
        );
        let models = ModelQuery::new(&files, Revision::new(2));
        let query = JavaQuery::new(&models, QueryScope::new(Revision::new(2), &NoSources));
        query.file(&source).unwrap().unwrap()
    };
    assert_eq!(
        actual.find_type(&name("Former"))[0].1.name(),
        Some("Former")
    );
    assert!(actual.find_type(&name("Latest")).is_empty());
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
    let models = ModelQuery::new(&files, revision);
    let query = JavaQuery::new(&models, QueryScope::new(revision, &Unrestricted));
    let found = query.find_type(&name("p.q.Outer.Member")).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source(), &source);
    assert_eq!(
        query.declaration(&found[0]).unwrap().unwrap().name(),
        Some("Member")
    );
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
    let models = ModelQuery::new(&files, revision);
    let query = JavaQuery::new(&models, QueryScope::new(revision, &Unrestricted));
    let found = query.find_type(&name("p.Example")).unwrap();
    assert_eq!(found.len(), 3);
    assert_ne!(found[0].source(), found[1].source());
    assert_ne!(found[0].source(), found[2].source());
    assert_ne!(found[1].source(), found[2].source());
    assert!(
        found
            .iter()
            .all(|entry| matches!(entry.source().root, ResourceRoot::File { .. }))
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
    let models = ModelQuery::new(&files, revision);
    let query = JavaQuery::new(&models, QueryScope::new(revision, &Unrestricted));
    assert_eq!(query.find_type(&name("p.Example")).unwrap().len(), 1);
}

#[test]
fn discovery_preserves_duplicate_declarations_within_one_resource() {
    let mut files = RevisionedStorage::default();
    let revision = Revision::new(1);
    files.put(
        revision,
        source_at("src/Example.java"),
        crate::lower_into("package p; class Example {} class Example {}"),
    );
    let models = ModelQuery::new(&files, revision);
    let query = JavaQuery::new(&models, QueryScope::new(revision, &Unrestricted));
    let found = query.find_type(&name("p.Example")).unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].source(), found[1].source());
    assert_ne!(found[0].node_index(), found[1].node_index());
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
    let old_models = ModelQuery::new(&files, Revision::new(1));
    let deleted_models = ModelQuery::new(&files, Revision::new(3));
    let old = JavaQuery::new(
        &old_models,
        QueryScope::new(Revision::new(1), &Unrestricted),
    );
    let deleted = JavaQuery::new(
        &deleted_models,
        QueryScope::new(Revision::new(3), &Unrestricted),
    );
    assert_eq!(old.find_type(&name("p.Example")).unwrap().len(), 1);
    assert!(deleted.find_type(&name("p.Example")).unwrap().is_empty());
    assert!(deleted.find_type(&name("p.Other")).unwrap().is_empty());
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
    let unrestricted = JavaQuery::new(&models, QueryScope::new(revision, &Unrestricted));
    let excluded = JavaQuery::new(&models, QueryScope::new(revision, &NoSources));
    assert_eq!(unrestricted.find_type(&name("p.Target")).unwrap().len(), 4);
    assert!(excluded.find_type(&name("p.Target")).unwrap().is_empty());
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
    let found = query.find_type(&name("p.Target")).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source(), &selected);
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
    let found = query.find_type(&name("Example")).unwrap();
    let current = &found[0];
    let older = DeclarationHandle::new(Revision::new(1), source, current.node_index());
    assert!(query.declaration(current).unwrap().is_some());
    assert!(query.declaration(&older).unwrap().is_none());
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
