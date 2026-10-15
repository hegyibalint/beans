use std::path::Path;

use beans_core::{
    classpath::Unrestricted,
    features::HoverRequest,
    location::{Location, LocationEntry},
    query_scope::QueryScope,
    ranges::ByteRange,
    revision::Revision,
};
use beans_lang_java::{model::File, query::JavaQuery};

use crate::Engine;

fn file(engine: &Engine, source: &Location) -> Option<File> {
    JavaQuery::new(
        &engine.storage,
        QueryScope::new(engine.revision(), &Unrestricted),
    )
    .file(source)
    .unwrap()
}

#[test]
fn accepts_only_supported_source_files() {
    let root = tempfile::tempdir().unwrap();
    let engine = Engine::new(root.path()).unwrap();
    assert!(engine.accept(Path::new("src/Example.java")));
    assert!(!engine.accept(Path::new("README.md")));
    assert!(!engine.accept(Path::new("lib/Example.class")));
}

#[test]
fn processing_java_source_commits_its_model_and_revision() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();
    let source = Location::file("/src/example/Example.java");
    engine
        .process(source.clone(), "package example; public class Example {}")
        .unwrap();
    assert_eq!(engine.revision(), Revision::new(1));
    assert_eq!(
        file(&engine, &source).unwrap().package_name.as_slice(),
        ["example"]
    );
}

#[test]
fn virtual_documents_keep_their_identity_through_replacement_and_removal() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();
    let source = Location::new(LocationEntry::Virtual("untitled:Example.java".into()));
    for (contents, end) in [
        ("class C extends First {}", 21),
        ("class C extends Second {}", 22),
    ] {
        engine
            .process_document(source.clone(), "java", contents)
            .unwrap();
        let response = engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: 16,
            })
            .unwrap()
            .unwrap();
        assert_eq!(response.range(), ByteRange::new(16, end));
        assert!(file(&engine, &Location::file("/Example.java")).is_none());
    }
    engine.close_document(source.clone()).unwrap();
    assert_eq!(engine.revision(), Revision::new(3));
    assert!(file(&engine, &source).is_none());
    let old = JavaQuery::new(
        &engine.storage,
        QueryScope::new(Revision::new(1), &Unrestricted),
    );
    assert!(old.file(&source).unwrap().is_some());
}

#[test]
fn unsupported_source_inputs_do_not_advance_the_revision() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();
    let virtual_source = Location::new(LocationEntry::Virtual("untitled:Example.java".into()));
    engine
        .process(Location::file("/README.md"), "not Java")
        .unwrap();
    engine
        .process(virtual_source.clone(), "class C extends T {}")
        .unwrap();
    engine
        .process(
            Location::file("/sources.jar").entry(LocationEntry::Container("Example.java".into())),
            "class C extends T {}",
        )
        .unwrap();
    engine
        .process_document(virtual_source.clone(), "plaintext", "class C extends T {}")
        .unwrap();
    assert_eq!(engine.revision(), Revision::default());
    assert!(file(&engine, &virtual_source).is_none());
}

#[test]
fn failed_publication_rolls_back_model_locations_and_indexes_without_advancing() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();
    let source = Location::file("/Example.java");
    engine.process(source.clone(), "class Original {}").unwrap();
    engine.storage.execute_batch("CREATE TRIGGER reject_type BEFORE INSERT ON java_types BEGIN SELECT RAISE(ABORT, 'type rejected'); END;").unwrap();
    let error = engine
        .process(source.clone(), "class Replacement {}")
        .unwrap_err();
    assert!(error.contains("type rejected"));
    assert_eq!(engine.revision(), Revision::new(1));
    let query = JavaQuery::new(
        &engine.storage,
        QueryScope::new(engine.revision(), &Unrestricted),
    );
    assert!(query.file(&source).unwrap().is_some());
    for table in ["entries", "locations", "java_types"] {
        let count: i64 = engine
            .storage
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}

#[test]
fn reopening_resumes_revisions_and_reads_persisted_models() {
    let root = tempfile::tempdir().unwrap();
    let source = Location::file("/Example.java");
    {
        let mut engine = Engine::new(root.path()).unwrap();
        engine.process(source.clone(), "class Original {}").unwrap();
    }
    let mut engine = Engine::new(root.path()).unwrap();
    assert_eq!(engine.revision(), Revision::new(1));
    assert!(file(&engine, &source).is_some());
    engine.process(source, "class Replacement {}").unwrap();
    assert_eq!(engine.revision(), Revision::new(2));
}

#[test]
fn revision_overflow_is_a_recoverable_processing_error() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();
    engine
        .storage
        .execute(
            "INSERT INTO entries (revision, model_kind, model) VALUES (?1, 0, '{}')",
            [i64::MAX],
        )
        .unwrap();
    assert_eq!(
        engine
            .process(Location::file("/Example.java"), "class Example {}")
            .unwrap_err(),
        "revision overflow"
    );
    assert_eq!(engine.revision(), Revision::default());
}
