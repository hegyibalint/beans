use beans_testing::{TempDir, support::class_bytes};
use lsp_server::{ErrorCode, Message, Response};
use serde_json::{Value, json};

mod support;
use support::{Client, assert_error, initialize_params, request};

#[test]
fn progress_waits_for_creation_and_only_uses_accepted_tokens() {
    for (accept_jdk, accept_failed_jdk) in
        [(true, true), (true, false), (false, true), (false, false)]
    {
        let root = TempDir::new();
        root.write("jdk with spaces/lib/modules", class_bytes("p/Runtime"));
        root.write(
            "beans.toml",
            "[unit.working]\njdk_home = 'jdk with spaces'\n[unit.missing]\njdk_home = 'missing'\n",
        );
        let mut params = initialize_params(root.root());
        params["capabilities"] = json!({"window": {"workDoneProgress": true}});
        let mut client = Client::default();
        assert!(request(&mut client, "initialize", params).error.is_none());

        for (accepted, path) in [
            (accept_jdk, root.path("jdk with spaces")),
            (accept_failed_jdk, root.path("missing")),
        ] {
            let Message::Request(create) = client.receive() else {
                panic!("expected progress creation request");
            };
            assert_eq!(create.method, "window/workDoneProgress/create");
            let token = create.params["token"].clone();
            assert!(token.as_str().unwrap().starts_with("beans:"));
            client.send(Response::new_ok("unrelated".to_string().into(), ()));
            assert_error(
                request(&mut client, "$/unknown", Value::Null),
                ErrorCode::MethodNotFound,
            );

            if accepted {
                client.send(Response::new_ok(create.id, ()));
                let Message::Notification(begin) = client.receive() else {
                    panic!("expected progress begin");
                };
                assert_eq!(begin.method, "$/progress");
                assert_eq!(
                    begin.params,
                    json!({
                        "token": token,
                        "value": {"kind": "begin", "title": "Processing JDK", "message": path.display().to_string(), "cancellable": false}
                    })
                );
                let Message::Notification(end) = client.receive() else {
                    panic!("expected progress end");
                };
                assert_eq!(end.method, "$/progress");
                assert_eq!(
                    end.params,
                    json!({"token": token, "value": {"kind": "end"}})
                );
            } else {
                client.send(Response::new_err(create.id, -32601, "unsupported".into()));
            }
        }
        assert!(
            request(&mut client, "shutdown", Value::Null)
                .error
                .is_none()
        );
        client.exit().unwrap();

        let connection = beans_engine::storage::open(&root.path(".beans/models")).unwrap();
        let name: String = connection
            .query_row(
                "SELECT json_extract(model, '$.name') FROM models WHERE model_kind = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "p/Runtime");
    }
}

#[test]
fn exit_releases_a_notification_worker_waiting_for_creation() {
    let root = TempDir::new();
    root.write("jdk/lib/modules", class_bytes("p/Runtime"));
    root.write("beans.toml", "jdk_home = 'jdk'\n[unit.app]\n");
    let mut params = initialize_params(root.root());
    params["capabilities"] = json!({"window": {"workDoneProgress": true}});
    let mut client = Client::default();
    assert!(request(&mut client, "initialize", params).error.is_none());
    let Message::Request(create) = client.receive() else {
        panic!("expected progress creation request");
    };
    assert_eq!(create.method, "window/workDoneProgress/create");

    assert!(
        request(&mut client, "shutdown", Value::Null)
            .error
            .is_none()
    );
    client.exit().unwrap();
}

#[test]
fn progress_is_not_requested_without_client_support() {
    let root = TempDir::new();
    root.write("jdk/lib/modules", class_bytes("p/Runtime"));
    root.write("beans.toml", "jdk_home = 'jdk'\n[unit.app]\n");
    let mut client = Client::default();
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
    assert!(
        request(&mut client, "shutdown", Value::Null)
            .error
            .is_none()
    );
    client.exit().unwrap();
}
