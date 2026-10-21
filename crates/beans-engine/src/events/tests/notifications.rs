use std::{fs, time::Duration};

use beans_core::event::{Command, Notification};
use beans_testing::support::class_bytes;
use crossbeam_channel::TryRecvError;
use tempfile::tempdir;

use crate::Engine;

#[test]
fn subscribers_receive_identical_pairs_on_success_and_failure() {
    let root = tempdir().unwrap();
    let resource = root.path().join("resource.class");
    fs::write(&resource, class_bytes("example/Resource")).unwrap();
    let home = root.path().join("jdk");
    fs::create_dir_all(home.join("lib")).unwrap();
    fs::write(home.join("lib/modules"), class_bytes("example/Runtime")).unwrap();
    let operations = [
        (false, root.path().join("missing.class")),
        (false, resource),
        (true, root.path().join("missing-jdk")),
        (true, home),
    ];
    let engine = Engine::start(root.path()).unwrap();
    let first = engine.subscribe();
    let second = engine.subscribe();

    for (is_jdk, path) in &operations {
        let command = if *is_jdk {
            Command::ProcessJdkHome(path.clone())
        } else {
            Command::ProcessResource(path.clone())
        };
        engine.sender().send(command).unwrap();
    }
    drop(engine);

    for notifications in [first, second] {
        for (index, (is_jdk, path)) in operations.iter().enumerate() {
            let start = notifications.try_recv().unwrap();
            let stop = notifications.try_recv().unwrap();
            let (start, stop) = match (*is_jdk, start, stop) {
                (
                    false,
                    Notification::ProcessResourceStart(start),
                    Notification::ProcessResourceStop(stop),
                ) => (start, stop),
                (
                    true,
                    Notification::ProcessJdkHomeStart(start),
                    Notification::ProcessJdkHomeStop(stop),
                ) => (start, stop),
                _ => panic!("expected matching processing start/stop notifications"),
            };
            assert_eq!(start.id, index as u64 + 1);
            assert_eq!(stop.id, start.id);
            assert_eq!(&start.data, path);
        }
        assert!(matches!(
            notifications.try_recv(),
            Err(TryRecvError::Disconnected)
        ));
    }
}

#[test]
fn new_subscribers_receive_future_notifications_without_replay() {
    let root = tempdir().unwrap();
    let engine = Engine::start(root.path()).unwrap();
    let first = engine.subscribe();
    engine
        .sender()
        .send(Command::ProcessResource(root.path().join("first")))
        .unwrap();
    for _ in 0..2 {
        first.recv_timeout(Duration::from_secs(5)).unwrap();
    }

    let second = engine.subscribe();
    assert!(second.is_empty());
    engine
        .sender()
        .send(Command::ProcessResource(root.path().join("second")))
        .unwrap();
    drop(engine);

    for notifications in [first, second] {
        let Notification::ProcessResourceStart(start) = notifications.try_recv().unwrap() else {
            panic!("expected ProcessResourceStart");
        };
        let Notification::ProcessResourceStop(stop) = notifications.try_recv().unwrap() else {
            panic!("expected ProcessResourceStop");
        };
        assert_eq!(start.id, 2);
        assert_eq!(stop.id, start.id);
        assert_eq!(start.data, root.path().join("second"));
        assert!(matches!(
            notifications.try_recv(),
            Err(TryRecvError::Disconnected)
        ));
    }
}

#[test]
fn disconnected_subscribers_are_removed_without_affecting_other_subscribers() {
    let root = tempdir().unwrap();
    let mut engine = Engine::start(root.path()).unwrap();
    let connected = engine.subscribe();
    drop(engine.subscribe());
    engine
        .sender()
        .send(Command::ProcessResource(root.path().join("missing")))
        .unwrap();

    engine.stop_command_loop().unwrap();

    assert_eq!(engine.subscribers.lock().unwrap().len(), 1);
    assert!(matches!(
        connected.try_recv().unwrap(),
        Notification::ProcessResourceStart(_)
    ));
    assert!(matches!(
        connected.try_recv().unwrap(),
        Notification::ProcessResourceStop(_)
    ));
    assert!(connected.is_empty());
}
