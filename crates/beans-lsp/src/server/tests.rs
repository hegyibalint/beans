mod diagnostics;
mod goto_definition;
mod hover;
mod lifecycle;
mod semantic_tokens;
mod storage;
mod workspace_import;

use super::{Lifecycle, Server};
use beans_testing::template::Template;
use lsp_server::{ErrorCode, Request, Response};
use serde_json::{Value, json};

fn position(fixture: &Template, offset: usize) -> Value {
    let before = &fixture.content[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count();
    let character = before
        .rsplit_once('\n')
        .map_or(before, |(_, current_line)| current_line)
        .encode_utf16()
        .count();
    json!({"line": line, "character": character})
}

fn server() -> (tempfile::TempDir, Server) {
    let (root, features) = crate::features::test_features();
    let server = Server {
        lifecycle: Lifecycle::Running,
        features: Some(features),
        notification_thread: None,
        notification_replies: None,
    };
    (root, server)
}

fn initialize_params(root: &std::path::Path) -> Value {
    json!({
        "capabilities": {},
        "workspaceFolders": [{
            "uri": url::Url::from_directory_path(root).unwrap().as_str(),
            "name": "test"
        }]
    })
}

fn request(server: &mut Server, method: &str, params: Value) -> Response {
    let response = server.respond(Request::new(7.into(), method.into(), params));
    assert_eq!(response.id, 7.into());
    response
}

fn assert_error(response: Response, code: ErrorCode) {
    assert_eq!(response.error.unwrap().code, code as i32);
    assert!(response.result.is_none());
}
