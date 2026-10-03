use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use lsp_types::InitializeParams;
use serde_json::json;

use super::super::workspace_roots;

#[test]
fn workspace_folders_supersede_the_legacy_root() {
    let params: InitializeParams = serde_json::from_value(json!({
        "capabilities": {},
        "rootUri": "file:///legacy",
        "workspaceFolders": [
            {"uri": "file:///one", "name": "one"},
            {"uri": "file:///two", "name": "two"},
            {"uri": "file:///one", "name": "duplicate"},
        ]
    }))
    .unwrap();

    assert_eq!(
        workspace_roots(&params),
        BTreeSet::from([PathBuf::from("/one"), PathBuf::from("/two")])
    );
}

#[test]
fn an_explicit_empty_folder_list_does_not_fall_back_to_root_uri() {
    // LSP 3.17 #workspace_workspaceFolders distinguishes an empty workspace
    // from a single-file session; workspaceFolders replaces rootUri.
    let params: InitializeParams = serde_json::from_value(json!({
        "capabilities": {}, "rootUri": "file:///legacy", "workspaceFolders": []
    }))
    .unwrap();

    assert!(workspace_roots(&params).is_empty());
}

#[test]
fn root_uri_is_used_when_workspace_folders_are_absent_or_null() {
    for value in [
        json!({"capabilities": {}, "rootUri": "file:///legacy"}),
        json!({"capabilities": {}, "rootUri": "file:///legacy", "workspaceFolders": null}),
    ] {
        let params: InitializeParams = serde_json::from_value(value).unwrap();
        assert_eq!(
            workspace_roots(&params),
            BTreeSet::from([PathBuf::from("/legacy")])
        );
    }
}

#[test]
fn non_file_folders_are_not_scanned_as_local_paths() {
    for uri in ["ssh://host/project", "beans-jvm:///project"] {
        let params: InitializeParams = serde_json::from_value(json!({
            "capabilities": {},
            "rootUri": "file:///legacy",
            "workspaceFolders": [{"uri": uri, "name": "remote"}],
        }))
        .unwrap();

        assert!(workspace_roots(&params).is_empty(), "{uri}");
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
        workspace_roots(&params),
        BTreeSet::from([Path::new("/my project").to_path_buf()])
    );
}
