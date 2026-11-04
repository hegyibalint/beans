use std::sync::Arc;

use beans_engine::Engine;
use beans_sidecar::Sidecar;
use beans_testing::TempDir;

#[test]
fn reader_dispatches_registration_to_the_shared_engine() {
    let files = TempDir::new();
    let engine = Arc::new(Engine::new(files.root()).unwrap());
    let mut sidecar = Sidecar::default();

    sidecar.start(Arc::downgrade(&engine)).unwrap();

    assert_eq!(
        sidecar.stop().unwrap_err(),
        "workspace registration is not implemented"
    );
    assert_eq!(Arc::strong_count(&engine), 1);
    sidecar.stop().unwrap();
}

#[test]
fn reader_does_not_keep_a_dropped_engine_alive() {
    let files = TempDir::new();
    let engine = Arc::new(Engine::new(files.root()).unwrap());
    let handle = Arc::downgrade(&engine);
    drop(engine);
    let mut sidecar = Sidecar::default();

    sidecar.start(handle.clone()).unwrap();

    assert_eq!(sidecar.stop().unwrap_err(), "engine is no longer available");
    assert!(handle.upgrade().is_none());
}
