use std::thread;

use beans_core::{
    events::{Event, JavaHomeFound},
    revision::Revision,
};

use super::Engine;

#[test]
fn a_new_engine_starts_at_the_default_revision() {
    assert_eq!(Engine::new().revision(), Revision::default());
}

#[test]
fn producers_share_one_engine_inbox() {
    let engine = Engine::new();
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
        let Event::JavaHomeFound(found) = engine.events.try_recv().unwrap();
        assert_eq!(found.home, std::path::Path::new(expected));
    }
    assert!(engine.events.try_recv().is_err());
}

#[test]
fn a_producer_can_send_without_borrowing_the_engine() {
    let engine = Engine::new();
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

    let Event::JavaHomeFound(found) = engine.events.try_recv().unwrap();
    assert_eq!(found.home, std::path::Path::new("/jdk"));
}
