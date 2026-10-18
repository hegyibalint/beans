use std::thread;

use beans_core::events::{Event, JavaHomeFound};

use super::Engine;

mod environment;

#[test]
fn an_engine_creates_and_owns_the_workspace_cache() {
    let root = tempfile::tempdir().unwrap();
    let engine = Engine::new(root.path()).unwrap();

    assert!(root.path().join(".beans/models").is_file());
    let models: i64 = engine
        .storage
        .query_row("SELECT COUNT(*) FROM models", [], |row| row.get(0))
        .unwrap();
    assert_eq!(models, 0);
}

#[test]
fn recreating_an_engine_preserves_the_workspace_cache() {
    let root = tempfile::tempdir().unwrap();
    {
        let engine = Engine::new(root.path()).unwrap();
        engine
            .storage
            .execute("INSERT INTO models VALUES (1, '[]', 0, '{}', NULL)", [])
            .unwrap();
    }
    let engine = Engine::new(root.path()).unwrap();
    let models: i64 = engine
        .storage
        .query_row("SELECT COUNT(*) FROM models", [], |row| row.get(0))
        .unwrap();
    assert_eq!(models, 1);
}

#[test]
fn cache_directory_creation_errors_are_returned() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join(".beans");
    std::fs::write(&directory, "blocked").unwrap();

    let error = Engine::new(root.path()).err().unwrap();
    assert!(error.contains(&directory.display().to_string()));
}

#[test]
fn cache_opening_errors_are_returned() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(".beans/models");
    std::fs::create_dir_all(&path).unwrap();

    let error = Engine::new(root.path()).err().unwrap();
    assert!(error.contains(&path.display().to_string()));
}

#[test]
fn producers_share_one_engine_inbox() {
    let root = tempfile::tempdir().unwrap();
    let engine = Engine::new(root.path()).unwrap();
    let first = engine.sender();
    let second = engine.sender();

    first
        .send(Event::JavaHomeFound(JavaHomeFound {
            home: "/first-jdk".into(),
        }))
        .unwrap();
    second
        .send(Event::JavaHomeFound(JavaHomeFound {
            home: "/second-jdk".into(),
        }))
        .unwrap();

    for expected in ["/first-jdk", "/second-jdk"] {
        let Event::JavaHomeFound(found) = engine.events.try_recv().unwrap() else {
            panic!("expected JavaHomeFound");
        };
        assert_eq!(found.home, std::path::Path::new(expected));
    }
    assert!(engine.events.try_recv().is_err());
}

#[test]
fn a_producer_can_send_without_borrowing_the_engine() {
    let root = tempfile::tempdir().unwrap();
    let engine = Engine::new(root.path()).unwrap();
    let sender = engine.sender();
    thread::spawn(move || {
        sender
            .send(Event::JavaHomeFound(JavaHomeFound {
                home: "/jdk".into(),
            }))
            .unwrap();
    })
    .join()
    .unwrap();

    let Event::JavaHomeFound(found) = engine.events.try_recv().unwrap() else {
        panic!("expected JavaHomeFound");
    };
    assert_eq!(found.home, std::path::Path::new("/jdk"));
}
