#![cfg(unix)]

use std::{process::Command, sync::Arc};

use beans_engine::Engine;
use beans_sidecar::Sidecar;
use beans_testing::TempDir;

#[test]
fn reader_dispatches_registration_to_the_shared_engine() {
    let files = TempDir::new();
    let engine = Arc::new(Engine::new(files.root()).unwrap());
    let mut sidecar = Sidecar::default();

    let mut command = Command::new("sh");
    command.args(["-c", "read ignored; printf x"]);
    sidecar.start(Arc::downgrade(&engine), command).unwrap();

    assert_eq!(
        sidecar.stop().unwrap_err(),
        "workspace registration is not implemented"
    );
    assert_eq!(Arc::strong_count(&engine), 1);
    sidecar.stop().unwrap();
}

#[test]
fn a_reader_waiting_for_input_does_not_keep_the_engine_alive() {
    let files = TempDir::new();
    let engine = Arc::new(Engine::new(files.root()).unwrap());
    let handle = Arc::downgrade(&engine);
    let mut sidecar = Sidecar::default();
    let mut command = Command::new("sh");
    command.args(["-c", "read ignored; printf x"]);
    sidecar.start(handle.clone(), command).unwrap();

    drop(engine);
    assert!(handle.upgrade().is_none());

    assert_eq!(sidecar.stop().unwrap_err(), "engine is no longer available");
}
