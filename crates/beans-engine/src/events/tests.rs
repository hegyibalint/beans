use std::{fs::File, sync::Arc};

use beans_core::event::Command;
use beans_platform_jvm::JvmPlatform;
use tempfile::tempdir;

use crate::Engine;

#[test]
fn a_new_engine_has_no_thread_and_cannot_be_stopped() {
    let root = tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();

    assert!(engine.command_loop.is_none());
    assert_eq!(engine.stop().unwrap_err(), "engine is not started");
}

#[test]
fn start_returns_an_engine_with_a_live_command_loop() {
    let root = tempdir().unwrap();
    let mut engine = Engine::start(root.path()).unwrap();

    assert!(!engine.command_loop.as_ref().unwrap().is_finished());
    assert!(root.path().join(".beans/models").is_file());
    engine.stop().unwrap();
}

#[test]
fn stopping_joins_and_clears_the_thread() {
    let root = tempdir().unwrap();
    let mut engine = Engine::start(root.path()).unwrap();

    engine.stop().unwrap();

    assert!(engine.command_loop.is_none());
    assert!(engine.command_receiver.is_empty());
    assert_eq!(engine.stop().unwrap_err(), "engine is not started");
}

#[test]
fn stop_follows_commands_already_in_the_queue() {
    let root = tempdir().unwrap();
    let mut engine = Engine::start(root.path()).unwrap();
    engine
        .sender()
        .send(Command::ProcessJdkHome("/jdk".into()))
        .unwrap();

    engine.stop().unwrap();

    assert!(engine.command_receiver.is_empty());
}

#[test]
fn dropping_a_running_engine_joins_the_thread_despite_live_producers() {
    let root = tempdir().unwrap();
    let engine = Engine::start(root.path()).unwrap();
    let sender = engine.sender();

    drop(engine);

    assert!(sender.send(Command::Stop).is_err());
}

#[test]
fn the_worker_shares_the_engine_platform_and_releases_it_on_shutdown() {
    let root = tempdir().unwrap();
    let mut engine = Engine::start(root.path()).unwrap();
    let jvm = Arc::downgrade(&engine.jvm);
    assert_eq!(jvm.strong_count(), 2);

    engine.stop().unwrap();
    assert_eq!(jvm.strong_count(), 1);
    drop(engine);
    assert_eq!(jvm.strong_count(), 0);
}

#[test]
fn a_failed_log_write_does_not_stop_the_command_loop() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let read_only = File::open(file.path()).unwrap();
    let (sender, commands) = crossbeam_channel::unbounded();
    let remaining = commands.clone();
    sender.send(Command::ProcessJdkHome("/jdk".into())).unwrap();
    sender.send(Command::Stop).unwrap();

    let root = tempdir().unwrap();
    let storage = crate::storage::open(&root.path().join("models")).unwrap();
    Engine::run(&JvmPlatform::new(), commands, Some(read_only), storage);

    assert!(remaining.is_empty());
    assert_eq!(file.as_file().metadata().unwrap().len(), 0);
}

mod ingestion;
