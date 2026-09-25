mod goto_definition;
mod hover;
mod lifecycle;

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

fn server(lifecycle: Lifecycle) -> Server {
    Server {
        lifecycle,
        ..Server::default()
    }
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
