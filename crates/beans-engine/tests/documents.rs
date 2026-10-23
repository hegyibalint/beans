use std::path::Path;

use beans_core::{
    classpath::Unrestricted,
    features::HoverRequest,
    location::{Location, LocationEntry},
    query_scope::QueryScope,
    ranges::ByteRange,
};
use beans_engine::{Engine, storage};
use beans_lang_java::{model::File, query::JavaQuery};
use beans_testing::TempDir;
use rusqlite::Connection;

#[test]
fn accepts_only_supported_source_files() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    assert!(engine.accept(Path::new("src/Example.java")));
    assert!(!engine.accept(Path::new("README.md")));
    assert!(!engine.accept(Path::new("lib/Example.class")));
}

#[test]
fn processing_java_source_commits_its_model() {
    let files = TempDir::new();
    let mut engine = Engine::new(files.root()).unwrap();
    let source = Location::file("/src/example/Example.java");
    assert!(
        engine
            .process(source.clone(), "package example; public class Example {}")
            .unwrap()
    );
    let cache = storage::open(&files.path(".beans/models")).unwrap();
    assert_eq!(
        file(&cache, &source).unwrap().package_name.as_slice(),
        ["example"]
    );
}

#[test]
fn virtual_documents_keep_their_identity_through_replacement_and_removal() {
    let files = TempDir::new();
    let mut engine = Engine::new(files.root()).unwrap();
    let cache = storage::open(&files.path(".beans/models")).unwrap();
    let source = Location::new(LocationEntry::Virtual("untitled:Example.java".into()));
    for (contents, end) in [
        ("class C extends First {}", 21),
        ("class C extends Second {}", 22),
    ] {
        assert!(
            engine
                .process_document(source.clone(), "java", contents)
                .unwrap()
        );
        let response = engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: 16,
            })
            .unwrap()
            .unwrap();
        assert_eq!(response.range(), ByteRange::new(16, end));
        assert!(file(&cache, &Location::file("/Example.java")).is_none());
        assert_eq!(count(&cache, "models"), 1);
    }
    assert!(engine.close_document(source.clone()).unwrap());
    assert!(file(&cache, &source).is_none());
    assert!(!engine.close_document(source).unwrap());
    for table in ["models", "java_types"] {
        assert_eq!(count(&cache, table), 0);
    }
}

#[test]
fn unsupported_inputs_report_no_change_and_write_nothing() {
    let files = TempDir::new();
    let mut engine = Engine::new(files.root()).unwrap();
    let virtual_source = Location::new(LocationEntry::Virtual("untitled:Example.java".into()));
    assert!(
        !engine
            .process(Location::file("/README.md"), "not Java")
            .unwrap()
    );
    assert!(
        !engine
            .process(virtual_source.clone(), "class C extends T {}")
            .unwrap()
    );
    assert!(
        !engine
            .process(
                Location::file("/sources.jar")
                    .entry(LocationEntry::Container("Example.java".into())),
                "class C extends T {}"
            )
            .unwrap()
    );
    assert!(
        !engine
            .process_document(virtual_source, "plaintext", "class C extends T {}")
            .unwrap()
    );
    let cache = storage::open(&files.path(".beans/models")).unwrap();
    assert_eq!(count(&cache, "models"), 0);
}

#[test]
fn failed_publication_restores_the_previous_model_and_indexes() {
    let files = TempDir::new();
    let mut engine = Engine::new(files.root()).unwrap();
    let source = Location::file("/Example.java");
    engine.process(source.clone(), "class Original {}").unwrap();
    let cache = storage::open(&files.path(".beans/models")).unwrap();
    let original = serde_json::to_value(file(&cache, &source).unwrap()).unwrap();
    cache.execute_batch("CREATE TRIGGER reject_type BEFORE INSERT ON java_types BEGIN SELECT RAISE(ABORT, 'type rejected'); END;").unwrap();
    assert!(
        engine
            .process(source.clone(), "class Replacement {}")
            .unwrap_err()
            .contains("type rejected")
    );
    assert_eq!(
        serde_json::to_value(file(&cache, &source).unwrap()).unwrap(),
        original
    );
    let query = JavaQuery::new(&cache, QueryScope::new(&Unrestricted));
    assert_eq!(
        query
            .find_type(&["Original".to_owned()].into_iter().collect())
            .unwrap()
            .len(),
        1
    );
    assert!(
        query
            .find_type(&["Replacement".to_owned()].into_iter().collect())
            .unwrap()
            .is_empty()
    );
    for table in ["models", "java_types"] {
        assert_eq!(count(&cache, table), 1);
    }
}

#[test]
fn failed_deletion_preserves_the_model_and_its_dependents() {
    let files = TempDir::new();
    let mut engine = Engine::new(files.root()).unwrap();
    let source = Location::file("/Example.java");
    engine.process(source.clone(), "class Example {}").unwrap();
    let cache = storage::open(&files.path(".beans/models")).unwrap();
    cache.execute_batch("CREATE TRIGGER reject_type_deletion BEFORE DELETE ON java_types BEGIN SELECT RAISE(ABORT, 'deletion rejected'); END;").unwrap();
    assert!(
        engine
            .close_document(source.clone())
            .unwrap_err()
            .contains("deletion rejected")
    );
    assert!(file(&cache, &source).is_some());
    for table in ["models", "java_types"] {
        assert_eq!(count(&cache, table), 1);
    }
}

#[test]
fn reopening_reads_and_replaces_the_persisted_model() {
    let files = TempDir::new();
    let source = Location::file("/Example.java");
    {
        let mut engine = Engine::new(files.root()).unwrap();
        engine.process(source.clone(), "class Original {}").unwrap();
    }
    let mut engine = Engine::new(files.root()).unwrap();
    let cache = storage::open(&files.path(".beans/models")).unwrap();
    assert!(file(&cache, &source).is_some());
    engine
        .process(source.clone(), "class Replacement {}")
        .unwrap();
    let query = JavaQuery::new(&cache, QueryScope::new(&Unrestricted));
    assert!(
        query
            .find_type(&["Original".to_owned()].into_iter().collect())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        query
            .find_type(&["Replacement".to_owned()].into_iter().collect())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(count(&cache, "models"), 1);
}

fn file(cache: &Connection, source: &Location) -> Option<File> {
    JavaQuery::new(cache, QueryScope::new(&Unrestricted))
        .file(source)
        .unwrap()
}

fn count(cache: &Connection, table: &str) -> i64 {
    cache
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}
