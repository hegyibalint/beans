use std::path::Path;

use beans_core::{
    artifact_sources::ArtifactSources, classpath::Classpath, location::Location, module::Module,
    resource_filter::ResourceFilter,
};
use beans_engine::{Engine, ImportKind, storage};
use beans_testing::TempDir;

#[test]
fn registration_round_trips_complete_modules_without_indexing_or_requiring_outputs() {
    let files = TempDir::new();
    let root = files.path("missing-build");
    let mut first = module(&root.join("App.java"));
    first.classpaths[0].entries = None;
    let modules = vec![first, module(&root.join("Lib.java"))];
    let expected = serde_json::to_value(&modules).unwrap();
    {
        let engine = Engine::new(files.root()).unwrap();
        engine
            .register_modules(modules, ImportKind::Gradle, &root)
            .unwrap();
    }

    let engine = Engine::new(files.root()).unwrap();
    assert_eq!(
        serde_json::to_value(engine.modules().unwrap()).unwrap(),
        expected
    );
    assert!(!root.exists());
    let connection = storage::open(&files.path(".beans/models")).unwrap();
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM models", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn snapshots_replace_and_clear_only_their_normalized_kind_and_root() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let a = files.path("a");
    let b = files.path("b");
    engine
        .register_modules(vec![module(&a.join("Toml.java"))], ImportKind::Toml, &a)
        .unwrap();
    engine
        .register_modules(
            vec![module(&a.join("Old.java")), module(&a.join("Other.java"))],
            ImportKind::Gradle,
            &a,
        )
        .unwrap();
    engine
        .register_modules(vec![module(&b.join("Keep.java"))], ImportKind::Gradle, &b)
        .unwrap();

    engine
        .register_modules(
            vec![module(&a.join("New.java"))],
            ImportKind::Gradle,
            &a.join("nested/.."),
        )
        .unwrap();
    let expected = vec![
        module(&a.join("Toml.java")),
        module(&b.join("Keep.java")),
        module(&a.join("New.java")),
    ];
    assert_eq!(
        serde_json::to_value(engine.modules().unwrap()).unwrap(),
        serde_json::to_value(expected).unwrap()
    );

    engine
        .register_modules(vec![], ImportKind::Gradle, &a)
        .unwrap();
    let expected = vec![module(&a.join("Toml.java")), module(&b.join("Keep.java"))];
    assert_eq!(
        serde_json::to_value(engine.modules().unwrap()).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}

#[test]
fn failed_insert_rolls_back_the_whole_snapshot_replacement() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    engine
        .register_modules(
            vec![module(&files.path("Old.java"))],
            ImportKind::Gradle,
            files.root(),
        )
        .unwrap();
    let expected = serde_json::to_value(engine.modules().unwrap()).unwrap();
    let connection = storage::open(&files.path(".beans/models")).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER reject_module BEFORE INSERT ON modules
             BEGIN SELECT RAISE(ABORT, 'intentional insert failure'); END;",
        )
        .unwrap();

    assert!(
        engine
            .register_modules(
                vec![module(&files.path("New.java"))],
                ImportKind::Gradle,
                files.root()
            )
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(engine.modules().unwrap()).unwrap(),
        expected
    );
}

#[test]
#[cfg(unix)]
fn serialization_failure_leaves_the_previous_snapshot_untouched() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    engine
        .register_modules(
            vec![module(&files.path("Old.java"))],
            ImportKind::Gradle,
            files.root(),
        )
        .unwrap();
    let expected = serde_json::to_value(engine.modules().unwrap()).unwrap();
    let mut invalid = module(&files.path("New.java"));
    invalid.artifact_sources[0].artifacts =
        vec![Location::file(OsString::from_vec(vec![b'/', 0xff]))];

    assert!(
        engine
            .register_modules(vec![invalid], ImportKind::Gradle, files.root())
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(engine.modules().unwrap()).unwrap(),
        expected
    );
}

fn module(source: &Path) -> Module {
    Module {
        classpaths: vec![Classpath {
            inputs: vec![ResourceFilter::file(source)],
            entries: Some(vec![]),
        }],
        artifact_sources: vec![ArtifactSources {
            artifacts: vec![Location::file(source.with_extension("jar"))],
            sources: vec![ResourceFilter::file(source)],
        }],
    }
}
