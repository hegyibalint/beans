use std::fs;

use serde_json::json;
use url::Url;

use super::*;

#[test]
fn overlapping_and_independent_workspace_folders_are_rejected_before_creating_caches() {
    let root = TempDir::new();
    let mut client = Client::default();
    for folders in [
        vec![root.root().to_path_buf(), root.path("nested")],
        vec![root.path("one"), root.path("two")],
    ] {
        for folder in &folders {
            fs::create_dir_all(folder).unwrap();
        }
        let workspace_folders: Vec<_> = folders
            .iter()
            .map(|path| {
                json!({
                    "uri": Url::from_directory_path(path).unwrap().as_str(), "name": "project"
                })
            })
            .collect();
        let response = request(
            &mut client,
            "initialize",
            json!({
                "capabilities": {}, "workspaceFolders": workspace_folders
            }),
        );
        let error = response.error.unwrap();
        assert_eq!(error.code, ErrorCode::RequestFailed as i32);
        assert_eq!(error.message, "expected exactly one workspace folder");
        for folder in &folders {
            assert!(!folder.join(".beans").exists());
        }
    }
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
}
