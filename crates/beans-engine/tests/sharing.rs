use std::{
    fs,
    sync::{Arc, Barrier},
    thread,
};

use beans_core::{features::HoverRequest, location::Location};
use beans_engine::Engine;
use beans_testing::TempDir;

#[test]
fn callers_share_one_engine_for_publication_and_queries() {
    let files = TempDir::new();
    let engine = Arc::new(Engine::new(files.root()).unwrap());
    let source = Location::file(files.path("Worker.java"));
    let contents = "class Worker extends Target {}";
    let barrier = Arc::new(Barrier::new(2));
    let writer = {
        let engine = Arc::clone(&engine);
        let source = source.clone();
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            barrier.wait();
            engine.process_document(source, "java", contents)
        })
    };

    barrier.wait();
    assert!(
        engine
            .process(Location::file(files.path("Main.java")), "class Main {}")
            .unwrap()
    );
    assert!(writer.join().unwrap().unwrap());
    let response = engine
        .feature_hover(&HoverRequest {
            source: &source,
            contents,
            offset: contents.find("Target").unwrap(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(response.contents().plaintext(), "Target");
}

#[test]
fn runtime_operations_do_not_recreate_a_missing_database() {
    let files = TempDir::new();
    let engine = Engine::new(files.root()).unwrap();
    let path = files.path(".beans/models");
    fs::remove_file(&path).unwrap();
    let source = Location::file(files.path("Example.java"));

    assert!(engine.diagnostics(&source).is_err());
    assert!(engine.process(source, "class Example {}").is_err());
    assert!(!path.exists());
}
