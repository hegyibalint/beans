use lsp_types::InitializeParams;
use serde_json::json;

use crate::features::Features;

#[test]
fn initialization_preserves_all_client_capabilities() {
    let root = tempfile::tempdir().unwrap();
    let params = InitializeParams {
        workspace_folders: Some(vec![lsp_types::WorkspaceFolder {
            uri: url::Url::from_directory_path(root.path())
                .unwrap()
                .as_str()
                .parse()
                .unwrap(),
            name: "test".into(),
        }]),
        capabilities: serde_json::from_value(json!({
            "workspace": {"workspaceFolders": true, "configuration": true},
            "textDocument": {
                "definition": {"linkSupport": true},
                "hover": {"contentFormat": ["markdown", "plaintext"]}
            },
            "window": {"workDoneProgress": true},
            "general": {"positionEncodings": ["utf-16"]},
            "experimental": {"beans": {"enabled": true}}
        }))
        .unwrap(),
        ..InitializeParams::default()
    };
    let mut features = Features::default();

    features.initialize(&params).unwrap();

    assert_eq!(features.client_capabilities, params.capabilities);
}
