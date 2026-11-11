use lsp_server::ErrorCode;
use serde_json::json;

use super::support::{assert_error, request, server};

#[test]
fn invalid_commands_are_rejected_without_stopping_the_server() {
    let (_files, mut client) = server();
    for params in [
        json!(null),
        json!({"command": "unknown"}),
        json!({"command": "beans.import.gradle", "arguments": ["unexpected"]}),
    ] {
        assert_error(
            request(&mut client, "workspace/executeCommand", params),
            ErrorCode::InvalidParams,
        );
    }
    assert!(
        request(&mut client, "shutdown", json!(null))
            .error
            .is_none()
    );
    client.exit().unwrap();
}

#[test]
fn import_reports_missing_configuration_without_starting_java() {
    let (_files, mut client) = server();
    let response = request(
        &mut client,
        "workspace/executeCommand",
        json!({
            "command": "beans.import.gradle", "arguments": []
        }),
    );
    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::RequestFailed as i32);
    if cfg!(feature = "sidecar") {
        assert!(error.message.contains("initializationOptions.sidecarJar"));
    } else {
        assert!(error.message.contains("--features sidecar"));
    }
    assert!(
        request(&mut client, "shutdown", json!(null))
            .error
            .is_none()
    );
    client.exit().unwrap();
}
