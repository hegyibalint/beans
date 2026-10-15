mod lifecycle;
mod workspace_import;

use super::{Lifecycle, Server};
use lsp_server::{ErrorCode, Request, Response};
use serde_json::Value;

fn server(lifecycle: Lifecycle) -> Server {
    Server {
        lifecycle,
        ..Server::default()
    }
}

fn request(server: &mut Server, method: &str, mut params: Value) -> Response {
    if method == "initialize"
        && params.is_object()
        && params.get("workspaceFolders").is_none()
        && params.get("rootUri").is_none()
    {
        let root = tempfile::tempdir().unwrap();
        let uri = url::Url::from_directory_path(root.path()).unwrap();
        params["workspaceFolders"] = serde_json::json!([{"uri": uri.as_str(), "name": "test"}]);
        server.test_workspace = Some(root);
    }
    let response = server
        .respond(Request::new(7.into(), method.into(), params))
        .unwrap();
    assert_eq!(response.id, 7.into());
    response
}

#[test]
fn definition_returns_scalar_locations_and_feature_failures_are_request_errors() {
    use lsp_server::Notification;
    let mut server = Server::default();
    request(
        &mut server,
        "initialize",
        serde_json::json!({"capabilities": {}}),
    );
    let source = "untitled:Source";
    let target = "untitled:Target";
    for (uri, text) in [
        (source, "import q.Target; class Use { Target field; }"),
        (target, "package q; class Target {}"),
    ] {
        server.notify(Notification::new("textDocument/didOpen".into(), serde_json::json!({"textDocument": {"uri": uri, "languageId": "java", "version": 1, "text": text}})));
    }
    let params = serde_json::json!({"textDocument": {"uri": source}, "position": {"line": 0, "character": 29}});
    let definition = request(&mut server, "textDocument/definition", params.clone());
    assert_eq!(definition.result.unwrap()["uri"], target);
    assert_error(
        request(&mut server, "textDocument/hover", Value::Null),
        ErrorCode::InvalidParams,
    );
    let database = server
        .test_workspace
        .as_ref()
        .unwrap()
        .path()
        .join(".beans/index.sqlite");
    std::fs::remove_file(database).unwrap();
    assert_error(
        request(&mut server, "textDocument/hover", params),
        ErrorCode::RequestFailed,
    );
}

fn assert_error(response: Response, code: ErrorCode) {
    assert_eq!(response.error.unwrap().code, code as i32);
    assert!(response.result.is_none());
}
