use std::{fs, io::Write, path::Path};

use beans_core::{
    classpath::Unrestricted,
    event::Command,
    location::{Location, LocationEntry},
    query_scope::QueryScope,
};
use beans_platform_jvm::{Class, query::JvmQuery, storage};
use beans_testing::support::class_bytes;
use tempfile::tempdir;
use zip::{ZipWriter, write::SimpleFileOptions};

use crate::Engine;

#[test]
fn a_resource_event_publishes_replacement_classes_before_stop() {
    let root = tempdir().unwrap();
    let path = root.path().join("resource");
    fs::write(&path, class_bytes("p/New")).unwrap();
    let mut engine = Engine::start(root.path()).unwrap();
    seed(&mut engine, "p/Old", &path);
    assert_eq!(classes(&engine, "p/Old"), [Location::file(&path)]);
    assert!(classes(&engine, "p/New").is_empty());

    enqueue(&engine, &path);
    engine.stop_command_loop().unwrap();

    assert!(classes(&engine, "p/Old").is_empty());
    assert_eq!(classes(&engine, "p/New"), [Location::file(&path)]);
}

#[test]
fn declined_resources_and_failures_preserve_old_classes_and_do_not_stop_later_events() {
    for contents in [
        Some(b"not a recognized resource".to_vec()),
        None,
        Some(class_bytes("p/Rejected")),
    ] {
        let root = tempdir().unwrap();
        let path = root.path().join("resource");
        if let Some(contents) = contents {
            fs::write(&path, contents).unwrap();
        }
        let good = root.path().join("good");
        fs::write(&good, class_bytes("p/Good")).unwrap();
        let mut engine = Engine::start(root.path()).unwrap();
        seed(&mut engine, "p/Old", &path);
        engine
            .storage
            .execute_batch(
                "CREATE TRIGGER reject_class BEFORE INSERT ON models
                 WHEN NEW.model_kind = 0 AND json_extract(NEW.model, '$.name') = 'p/Rejected'
                 BEGIN SELECT RAISE(ABORT, 'rejected class'); END;",
            )
            .unwrap();
        enqueue(&engine, &path);
        enqueue(&engine, &good);

        engine.stop_command_loop().unwrap();

        assert_eq!(classes(&engine, "p/Old"), [Location::file(&path)]);
        assert!(classes(&engine, "p/Rejected").is_empty());
        assert_eq!(classes(&engine, "p/Good"), [Location::file(&good)]);
    }
}

#[test]
fn a_recognized_resource_with_no_valid_classes_removes_old_classes() {
    let root = tempdir().unwrap();
    let path = root.path().join("resource");
    fs::write(&path, [0xca, 0xfe, 0xba, 0xbe]).unwrap();
    let mut engine = Engine::start(root.path()).unwrap();
    seed(&mut engine, "p/Old", &path);
    enqueue(&engine, &path);

    engine.stop_command_loop().unwrap();

    assert!(classes(&engine, "p/Old").is_empty());
}

#[test]
fn a_java_home_event_publishes_runtime_and_tools_together() {
    let root = tempdir().unwrap();
    let home = root.path().join("jdk");
    let runtime = home.join("jre/lib/rt.jar");
    let tools = home.join("lib/tools.jar");
    write_jar(&runtime, "p/Runtime");
    write_jar(&tools, "p/Tool");
    let mut engine = Engine::start(root.path()).unwrap();
    engine
        .sender()
        .send(Command::ProcessJdkHome(home))
        .unwrap();

    engine.stop_command_loop().unwrap();

    assert_eq!(
        classes(&engine, "p/Runtime"),
        [Location::file(&runtime).entry(LocationEntry::Container("p/Runtime.class".into()))]
    );
    assert_eq!(
        classes(&engine, "p/Tool"),
        [Location::file(&tools).entry(LocationEntry::Container("p/Tool.class".into()))]
    );
}

#[test]
fn a_java_home_failure_rolls_back_all_artifacts_and_processing_continues() {
    let root = tempdir().unwrap();
    let home = root.path().join("jdk");
    let runtime = home.join("jre/lib/rt.jar");
    let tools = home.join("lib/tools.jar");
    write_jar(&runtime, "p/New");
    fs::create_dir_all(tools.parent().unwrap()).unwrap();
    fs::write(&tools, b"unrecognized tools").unwrap();
    let good = root.path().join("good");
    fs::write(&good, class_bytes("p/Good")).unwrap();
    let mut engine = Engine::start(root.path()).unwrap();
    seed(&mut engine, "p/OldRuntime", &runtime);
    seed(&mut engine, "p/OldTool", &tools);
    engine
        .sender()
        .send(Command::ProcessJdkHome(home))
        .unwrap();
    enqueue(&engine, &good);

    engine.stop_command_loop().unwrap();

    assert_eq!(classes(&engine, "p/OldRuntime"), [Location::file(&runtime)]);
    assert_eq!(classes(&engine, "p/OldTool"), [Location::file(&tools)]);
    assert!(classes(&engine, "p/New").is_empty());
    assert_eq!(classes(&engine, "p/Good"), [Location::file(&good)]);
}

#[test]
fn a_storage_open_failure_is_reported_by_start() {
    let root = tempdir().unwrap();
    fs::create_dir_all(root.path().join(".beans/models")).unwrap();

    assert!(Engine::start(root.path()).is_err());
}

fn enqueue(engine: &Engine, path: &Path) {
    engine
        .sender()
        .send(Command::ProcessResource(path.into()))
        .unwrap();
}

fn seed(engine: &mut Engine, name: &str, path: &Path) {
    let transaction = engine.storage.transaction().unwrap();
    storage::store(
        &transaction,
        &Class { name: name.into() },
        &Location::file(path),
    )
    .unwrap();
    transaction.commit().unwrap();
}

fn classes(engine: &Engine, name: &str) -> Vec<Location> {
    JvmQuery::new(&engine.storage, QueryScope::new(&Unrestricted))
        .find_classes(name)
        .unwrap()
}

fn write_jar(path: &Path, name: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut jar = ZipWriter::new(fs::File::create(path).unwrap());
    jar.start_file(format!("{name}.class"), SimpleFileOptions::default())
        .unwrap();
    jar.write_all(&class_bytes(name)).unwrap();
    jar.finish().unwrap();
}
