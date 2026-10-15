use std::path::PathBuf;

use lsp_types::InitializeParams;
use serde_json::json;

use super::super::workspace_root;

#[test]
fn workspace_folders_supersede_the_legacy_root() {
    let params: InitializeParams = serde_json::from_value(json!({
        "capabilities": {},
        "rootUri": "file:///legacy",
        "workspaceFolders": [{"uri": "file:///project", "name": "project"}]
    }))
    .unwrap();

    assert_eq!(workspace_root(&params).unwrap(), PathBuf::from("/project"));
}

#[test]
fn empty_missing_and_multiple_workspace_folders_are_rejected() {
    for value in [
        json!({"capabilities": {}}),
        json!({"capabilities": {}, "rootUri": null, "workspaceFolders": null}),
        json!({"capabilities": {}, "rootUri": "file:///legacy", "workspaceFolders": []}),
        json!({"capabilities": {}, "workspaceFolders": [
            {"uri": "file:///one", "name": "one"},
            {"uri": "file:///two", "name": "two"}
        ]}),
        json!({"capabilities": {}, "workspaceFolders": [
            {"uri": "file:///one", "name": "one"},
            {"uri": "file:///one", "name": "duplicate"}
        ]}),
        json!({"capabilities": {}, "workspaceFolders": [
            {"uri": "file:///one", "name": "one"},
            {"uri": "ssh://host/two", "name": "two"}
        ]}),
    ] {
        let params: InitializeParams = serde_json::from_value(value).unwrap();
        assert_eq!(
            workspace_root(&params).unwrap_err(),
            "expected exactly one workspace folder"
        );
    }
}

#[test]
fn root_uri_is_used_when_workspace_folders_are_absent_or_null() {
    for value in [
        json!({"capabilities": {}, "rootUri": "file:///legacy"}),
        json!({"capabilities": {}, "rootUri": "file:///legacy", "workspaceFolders": null}),
    ] {
        let params: InitializeParams = serde_json::from_value(value).unwrap();
        assert_eq!(workspace_root(&params).unwrap(), PathBuf::from("/legacy"));
    }
}

#[test]
fn non_file_roots_are_rejected_instead_of_using_the_legacy_root() {
    for uri in ["ssh://host/project", "beans-jvm:///project"] {
        let params: InitializeParams = serde_json::from_value(json!({
            "capabilities": {},
            "rootUri": "file:///legacy",
            "workspaceFolders": [{"uri": uri, "name": "remote"}],
        }))
        .unwrap();

        assert!(
            workspace_root(&params)
                .unwrap_err()
                .contains("local file URI")
        );
    }
}

#[test]
fn encoded_folder_uris_are_decoded_before_import() {
    let params: InitializeParams = serde_json::from_value(json!({
        "capabilities": {},
        "workspaceFolders": [{"uri": "file:///my%20project", "name": "project"}],
    }))
    .unwrap();

    assert_eq!(
        workspace_root(&params).unwrap(),
        PathBuf::from("/my project")
    );
}
