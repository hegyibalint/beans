use std::fs;

use serde_json::json;
use url::Url;

use super::*;

#[test]
fn invalid_descriptors_fail_initialization_with_the_descriptor_path() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("beans.toml"),
        "[module.main]\ndepends_on = [\"missing\"]\n",
    )
    .unwrap();
    let mut server = Server::default();
    let uri = Url::from_directory_path(root.path()).unwrap();

    let response = request(
        &mut server,
        "initialize",
        json!({
            "capabilities": {}, "workspaceFolders": [{"uri": uri.as_str(), "name": "project"}]
        }),
    );

    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::RequestFailed as i32);
    assert!(
        error
            .message
            .contains(&root.path().join("beans.toml").display().to_string())
    );
    assert!(matches!(server.lifecycle, Lifecycle::Uninitialized));
}
