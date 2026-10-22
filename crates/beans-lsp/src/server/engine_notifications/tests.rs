use std::{path::PathBuf, time::Duration};

use beans_core::event::NotificationPayload;
use serde_json::json;

use super::*;

#[test]
fn progress_waits_for_creation_and_pairs_tokens_without_claiming_success() {
    let (notifications, receiver) = unbounded();
    let (replies, responses) = unbounded();
    let (sender, outgoing) = unbounded();
    let worker = thread::spawn(move || {
        consume_notifications(receiver, responses, sender, RequestLog::default())
    });
    notifications
        .send(EngineNotification::ProcessResourceStop(
            NotificationPayload { id: 99, data: () },
        ))
        .unwrap();

    for (is_jdk, id, accepted) in [(false, 1, true), (false, 2, false), (true, u64::MAX, true)] {
        let path = PathBuf::from("/resource with spaces");
        let payload = NotificationPayload {
            id,
            data: path.clone(),
        };
        let stop = NotificationPayload { id, data: () };
        let (start, stop, title) = if is_jdk {
            (
                EngineNotification::ProcessJdkHomeStart(payload),
                EngineNotification::ProcessJdkHomeStop(stop),
                "Processing JDK",
            )
        } else {
            (
                EngineNotification::ProcessResourceStart(payload),
                EngineNotification::ProcessResourceStop(stop),
                "Processing resource",
            )
        };
        notifications.send(start).unwrap();
        notifications.send(stop).unwrap();
        let Message::Request(create) = outgoing.recv_timeout(Duration::from_secs(5)).unwrap()
        else {
            panic!("expected progress creation request");
        };
        assert_eq!(create.method, "window/workDoneProgress/create");
        assert_eq!(create.params, json!({"token": format!("beans:{id}")}));
        assert!(outgoing.is_empty());
        replies
            .send(Response::new_ok("unrelated".to_string().into(), ()))
            .unwrap();
        let response = if accepted {
            Response::new_ok(create.id, ())
        } else {
            Response::new_err(create.id, -32601, "unsupported".into())
        };
        replies.send(response).unwrap();
        if accepted {
            let Message::Notification(begin) =
                outgoing.recv_timeout(Duration::from_secs(5)).unwrap()
            else {
                panic!("expected progress begin");
            };
            assert_eq!(begin.method, "$/progress");
            assert_eq!(
                begin.params,
                json!({
                    "token": format!("beans:{id}"),
                    "value": {
                        "kind": "begin",
                        "title": title,
                        "message": path.display().to_string(),
                        "cancellable": false
                    }
                })
            );
            let Message::Notification(end) = outgoing.recv_timeout(Duration::from_secs(5)).unwrap()
            else {
                panic!("expected progress end");
            };
            assert_eq!(end.method, "$/progress");
            assert_eq!(
                end.params,
                json!({"token": format!("beans:{id}"), "value": {"kind": "end"}})
            );
        }
    }
    drop(notifications);
    drop(replies);
    worker.join().unwrap().unwrap();
    assert!(outgoing.try_recv().is_err());
}

#[test]
fn closing_the_reply_pipe_releases_a_worker_waiting_for_creation() {
    let (notifications, receiver) = unbounded();
    let (replies, responses) = unbounded();
    let (sender, outgoing) = unbounded();
    let worker = thread::spawn(move || {
        consume_notifications(receiver, responses, sender, RequestLog::default())
    });
    notifications
        .send(EngineNotification::ProcessResourceStart(
            NotificationPayload {
                id: 1,
                data: "/resource".into(),
            },
        ))
        .unwrap();
    assert!(matches!(
        outgoing.recv_timeout(Duration::from_secs(5)).unwrap(),
        Message::Request(_)
    ));

    drop(replies);

    worker.join().unwrap().unwrap();
    assert!(outgoing.try_recv().is_err());
    assert!(
        notifications
            .send(EngineNotification::ProcessResourceStop(
                NotificationPayload { id: 1, data: () }
            ))
            .is_err()
    );
}
