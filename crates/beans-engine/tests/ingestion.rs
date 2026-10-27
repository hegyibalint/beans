use std::{fs, path::Path};

use beans_core::{
    event::Command,
    location::{Location, LocationEntry},
    query_scope::QueryScope,
};
use beans_engine::{Engine, storage};
use beans_platform_jvm::{Class, query::JvmQuery, storage as jvm_storage};
use beans_testing::{
    TempDir,
    support::{class_bytes, jar_bytes},
};
use rusqlite::Connection;

#[test]
fn a_resource_event_publishes_replacement_classes_before_drop_returns() {
    let files = TempDir::new();
    let path = files.write("resource", class_bytes("p/New"));
    let engine = Engine::start(files.root()).unwrap();
    let mut cache = storage::open(&files.path(".beans/models")).unwrap();
    seed(&mut cache, "p/Old", &path);
    assert_eq!(classes(&cache, "p/Old"), [Location::file(&path)]);
    assert!(classes(&cache, "p/New").is_empty());

    enqueue(&engine, &path);
    drop(engine);

    assert!(classes(&cache, "p/Old").is_empty());
    assert_eq!(classes(&cache, "p/New"), [Location::file(&path)]);
}

#[test]
fn unsupported_resources_and_failures_preserve_old_classes_and_do_not_stop_later_events() {
    for contents in [
        Some(b"not a recognized resource".to_vec()),
        None,
        Some(class_bytes("p/Rejected")),
    ] {
        let files = TempDir::new();
        let path = files.path("resource");
        if let Some(contents) = contents {
            files.write("resource", contents);
        }
        let good = files.write("good", class_bytes("p/Good"));
        let engine = Engine::start(files.root()).unwrap();
        let mut cache = storage::open(&files.path(".beans/models")).unwrap();
        seed(&mut cache, "p/Old", &path);
        cache
            .execute_batch(
                "CREATE TRIGGER reject_class BEFORE INSERT ON models
                 WHEN NEW.kind = 0 AND json_extract(NEW.model, '$.name') = 'p/Rejected'
                 BEGIN SELECT RAISE(ABORT, 'rejected class'); END;",
            )
            .unwrap();
        enqueue(&engine, &path);
        enqueue(&engine, &good);

        drop(engine);

        assert_eq!(classes(&cache, "p/Old"), [Location::file(&path)]);
        assert!(classes(&cache, "p/Rejected").is_empty());
        assert_eq!(classes(&cache, "p/Good"), [Location::file(&good)]);
    }
}

#[test]
fn a_recognized_resource_with_no_valid_classes_removes_old_classes() {
    let files = TempDir::new();
    let path = files.write("resource", [0xca, 0xfe, 0xba, 0xbe]);
    let engine = Engine::start(files.root()).unwrap();
    let mut cache = storage::open(&files.path(".beans/models")).unwrap();
    seed(&mut cache, "p/Old", &path);
    enqueue(&engine, &path);

    drop(engine);

    assert!(classes(&cache, "p/Old").is_empty());
}

#[test]
fn a_java_home_event_publishes_runtime_and_tools_together() {
    let files = TempDir::new();
    let runtime = files.write(
        "jdk/jre/lib/rt.jar",
        jar_bytes(&[("p/Runtime.class", &class_bytes("p/Runtime"))]),
    );
    let tools = files.write(
        "jdk/lib/tools.jar",
        jar_bytes(&[("p/Tool.class", &class_bytes("p/Tool"))]),
    );
    let engine = Engine::start(files.root()).unwrap();
    engine
        .sender()
        .send(Command::ProcessJdkHome(files.path("jdk")))
        .unwrap();

    drop(engine);

    let cache = storage::open(&files.path(".beans/models")).unwrap();
    assert_eq!(
        classes(&cache, "p/Runtime"),
        [Location::file(&runtime).entry(LocationEntry::Container("p/Runtime.class".into()))]
    );
    assert_eq!(
        classes(&cache, "p/Tool"),
        [Location::file(&tools).entry(LocationEntry::Container("p/Tool.class".into()))]
    );
}

#[test]
fn a_java_home_failure_preserves_all_artifacts_and_processing_continues() {
    let files = TempDir::new();
    let runtime = files.write(
        "jdk/jre/lib/rt.jar",
        jar_bytes(&[("p/New.class", &class_bytes("p/New"))]),
    );
    let tools = files.write("jdk/lib/tools.jar", b"unrecognized tools");
    let good = files.write("good", class_bytes("p/Good"));
    let engine = Engine::start(files.root()).unwrap();
    let mut cache = storage::open(&files.path(".beans/models")).unwrap();
    seed(&mut cache, "p/OldRuntime", &runtime);
    seed(&mut cache, "p/OldTool", &tools);
    engine
        .sender()
        .send(Command::ProcessJdkHome(files.path("jdk")))
        .unwrap();
    enqueue(&engine, &good);

    drop(engine);

    assert_eq!(classes(&cache, "p/OldRuntime"), [Location::file(&runtime)]);
    assert_eq!(classes(&cache, "p/OldTool"), [Location::file(&tools)]);
    assert!(classes(&cache, "p/New").is_empty());
    assert_eq!(classes(&cache, "p/Good"), [Location::file(&good)]);
}

#[test]
fn a_storage_open_failure_is_reported_by_start() {
    let files = TempDir::new();
    fs::create_dir_all(files.path(".beans/models")).unwrap();

    assert!(Engine::start(files.root()).is_err());
}

fn enqueue(engine: &Engine, path: &Path) {
    engine
        .sender()
        .send(Command::ProcessResource(path.into()))
        .unwrap();
}

fn seed(cache: &mut Connection, name: &str, path: &Path) {
    let transaction = cache.transaction().unwrap();
    jvm_storage::store(
        &transaction,
        &Class { name: name.into() },
        &Location::file(path),
    )
    .unwrap();
    transaction.commit().unwrap();
}

fn classes(cache: &Connection, name: &str) -> Vec<Location> {
    JvmQuery::new(cache, QueryScope::unrestricted())
        .find_classes(name)
        .unwrap()
}
