use std::time::Duration;

use beans_core::event::{Command, Notification};
use beans_engine::Engine;
use beans_testing::{TempDir, support::class_bytes};
use crossbeam_channel::TryRecvError;

#[test]
fn subscribers_receive_identical_pairs_on_success_and_failure() {
    let files = TempDir::new();
    let resource = files.write("resource.class", class_bytes("example/Resource"));
    files.write("jdk/lib/modules", class_bytes("example/Runtime"));
    let operations = [
        (false, files.path("missing.class")),
        (false, resource),
        (true, files.path("missing-jdk")),
        (true, files.path("jdk")),
    ];
    let engine = Engine::start(files.root()).unwrap();
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
    let files = TempDir::new();
    let engine = Engine::start(files.root()).unwrap();
    let first = engine.subscribe();
    engine
        .sender()
        .send(Command::ProcessResource(files.path("first")))
        .unwrap();
    for _ in 0..2 {
        first.recv_timeout(Duration::from_secs(5)).unwrap();
    }

    let second = engine.subscribe();
    assert!(second.is_empty());
    engine
        .sender()
        .send(Command::ProcessResource(files.path("second")))
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
        assert_eq!(start.data, files.path("second"));
        assert!(matches!(
            notifications.try_recv(),
            Err(TryRecvError::Disconnected)
        ));
    }
}

#[test]
fn disconnected_subscribers_do_not_affect_other_subscribers() {
    let files = TempDir::new();
    let engine = Engine::start(files.root()).unwrap();
    let connected = engine.subscribe();
    drop(engine.subscribe());
    engine
        .sender()
        .send(Command::ProcessResource(files.path("missing")))
        .unwrap();

    drop(engine);

    assert!(matches!(
        connected.try_recv().unwrap(),
        Notification::ProcessResourceStart(_)
    ));
    assert!(matches!(
        connected.try_recv().unwrap(),
        Notification::ProcessResourceStop(_)
    ));
    assert!(matches!(
        connected.try_recv(),
        Err(TryRecvError::Disconnected)
    ));
}
