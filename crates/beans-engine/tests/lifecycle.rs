use std::thread;

use beans_core::event::{Command, Notification};
use beans_engine::Engine;
use beans_testing::{TempDir, support::class_bytes};
use crossbeam_channel::TryRecvError;

#[test]
fn a_new_engine_does_not_process_queued_commands() {
    let files = TempDir::new();
    let path = files.write("resource.class", class_bytes("example/Resource"));
    let engine = Engine::new(files.root()).unwrap();
    let notifications = engine.subscribe();
    engine
        .sender()
        .send(Command::ProcessResource(path))
        .unwrap();

    drop(engine);

    assert!(matches!(
        notifications.try_recv(),
        Err(TryRecvError::Disconnected)
    ));
}

#[test]
fn producers_share_one_engine_inbox() {
    let files = TempDir::new();
    let engine = Engine::start(files.root()).unwrap();
    let notifications = engine.subscribe();
    let first = engine.sender();
    let second = engine.sender();
    let homes = [files.path("first-jdk"), files.path("second-jdk")];
    first
        .send(Command::ProcessJdkHome(homes[0].clone()))
        .unwrap();
    second
        .send(Command::ProcessJdkHome(homes[1].clone()))
        .unwrap();

    drop(engine);

    for (index, expected) in homes.into_iter().enumerate() {
        let Notification::ProcessJdkHomeStart(start) = notifications.try_recv().unwrap() else {
            panic!("expected ProcessJdkHomeStart");
        };
        let Notification::ProcessJdkHomeStop(stop) = notifications.try_recv().unwrap() else {
            panic!("expected ProcessJdkHomeStop");
        };
        assert_eq!(start.id, index as u64 + 1);
        assert_eq!(start.data, expected);
        assert_eq!(stop.id, start.id);
    }
    assert!(matches!(
        notifications.try_recv(),
        Err(TryRecvError::Disconnected)
    ));
}

#[test]
fn a_producer_can_send_without_borrowing_the_engine() {
    let files = TempDir::new();
    let engine = Engine::start(files.root()).unwrap();
    let notifications = engine.subscribe();
    let sender = engine.sender();
    let home = files.path("jdk");
    let command = Command::ProcessJdkHome(home.clone());
    thread::spawn(move || sender.send(command).unwrap())
        .join()
        .unwrap();

    drop(engine);

    let Notification::ProcessJdkHomeStart(start) = notifications.try_recv().unwrap() else {
        panic!("expected ProcessJdkHomeStart");
    };
    let Notification::ProcessJdkHomeStop(stop) = notifications.try_recv().unwrap() else {
        panic!("expected ProcessJdkHomeStop");
    };
    assert_eq!(start.id, 1);
    assert_eq!(start.data, home);
    assert_eq!(stop.id, start.id);
    assert!(matches!(
        notifications.try_recv(),
        Err(TryRecvError::Disconnected)
    ));
}

#[test]
fn dropping_a_running_engine_joins_the_thread_despite_live_producers() {
    let files = TempDir::new();
    let engine = Engine::start(files.root()).unwrap();
    let sender = engine.sender();

    drop(engine);

    assert!(sender.send(Command::Stop).is_err());
}
