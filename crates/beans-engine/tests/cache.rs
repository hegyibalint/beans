use std::fs;

use beans_engine::{Engine, storage};
use beans_testing::TempDir;

#[test]
fn an_engine_creates_and_owns_the_workspace_cache() {
    let files = TempDir::new();
    let _engine = Engine::new(files.root()).unwrap();
    let path = files.path(".beans/models");
    assert!(path.is_file());
    let cache = storage::open(&path).unwrap();
    let models: i64 = cache
        .query_row("SELECT COUNT(*) FROM models", [], |row| row.get(0))
        .unwrap();
    assert_eq!(models, 0);
}

#[test]
fn recreating_an_engine_preserves_the_workspace_cache() {
    let files = TempDir::new();
    let path = files.path(".beans/models");
    {
        let _engine = Engine::new(files.root()).unwrap();
        let cache = storage::open(&path).unwrap();
        cache
            .execute("INSERT INTO models VALUES (1, '[]', 0, '{}', NULL)", [])
            .unwrap();
    }
    let _engine = Engine::new(files.root()).unwrap();
    let cache = storage::open(&path).unwrap();
    let models: i64 = cache
        .query_row("SELECT COUNT(*) FROM models", [], |row| row.get(0))
        .unwrap();
    assert_eq!(models, 1);
}

#[test]
fn cache_directory_creation_errors_are_returned() {
    let files = TempDir::new();
    let directory = files.write(".beans", "blocked");

    let error = Engine::new(files.root()).err().unwrap();
    assert!(error.contains(&directory.display().to_string()));
}

#[test]
fn cache_opening_errors_are_returned() {
    let files = TempDir::new();
    let path = files.path(".beans/models");
    fs::create_dir_all(&path).unwrap();

    let error = Engine::new(files.root()).err().unwrap();
    assert!(error.contains(&path.display().to_string()));
}
