use std::{fs, path::Path};

use beans_core::{
    event::Command,
    location::{Location, LocationEntry},
};
use beans_engine::{Engine, storage};
use beans_platform_jvm::{Class, query, storage as jvm_storage};
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
    seed("p/Old", &path, &mut cache);
    assert_eq!(classes("p/Old", &cache), [Location::file(&path)]);
    assert!(classes("p/New", &cache).is_empty());

    enqueue(&engine, &path);
    drop(engine);

    assert!(classes("p/Old", &cache).is_empty());
    assert_eq!(classes("p/New", &cache), [Location::file(&path)]);
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
        seed("p/Old", &path, &mut cache);
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

        assert_eq!(classes("p/Old", &cache), [Location::file(&path)]);
        assert!(classes("p/Rejected", &cache).is_empty());
        assert_eq!(classes("p/Good", &cache), [Location::file(&good)]);
    }
}

#[test]
fn a_recognized_resource_with_no_valid_classes_removes_old_classes() {
    let files = TempDir::new();
    let path = files.write("resource", [0xca, 0xfe, 0xba, 0xbe]);
    let engine = Engine::start(files.root()).unwrap();
    let mut cache = storage::open(&files.path(".beans/models")).unwrap();
    seed("p/Old", &path, &mut cache);
    enqueue(&engine, &path);

    drop(engine);

    assert!(classes("p/Old", &cache).is_empty());
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
        classes("p/Runtime", &cache),
        [Location::file(&runtime).entry(LocationEntry::Container("p/Runtime.class".into()))]
    );
    assert_eq!(
        classes("p/Tool", &cache),
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
    seed("p/OldRuntime", &runtime, &mut cache);
    seed("p/OldTool", &tools, &mut cache);
    engine
        .sender()
        .send(Command::ProcessJdkHome(files.path("jdk")))
        .unwrap();
    enqueue(&engine, &good);

    drop(engine);

    assert_eq!(classes("p/OldRuntime", &cache), [Location::file(&runtime)]);
    assert_eq!(classes("p/OldTool", &cache), [Location::file(&tools)]);
    assert!(classes("p/New", &cache).is_empty());
    assert_eq!(classes("p/Good", &cache), [Location::file(&good)]);
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

fn seed(name: &str, path: &Path, cache: &mut Connection) {
    let transaction = cache.transaction().unwrap();
    jvm_storage::store(
        &Class { name: name.into() },
        &Location::file(path),
        &transaction,
    )
    .unwrap();
    transaction.commit().unwrap();
}

fn classes(name: &str, cache: &Connection) -> Vec<Location> {
    query::find_classes(name, cache).unwrap()
}
