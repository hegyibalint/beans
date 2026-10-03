mod capabilities;
mod folder_selection;
mod imports;
mod roots;

use std::path::Path;

use lsp_types::InitializeParams;
use serde_json::json;
use url::Url;

fn params(roots: &[&Path]) -> InitializeParams {
    let folders: Vec<_> = roots
        .iter()
        .map(|root| {
            json!({
                "uri": Url::from_directory_path(root).unwrap().as_str(),
                "name": "project",
            })
        })
        .collect();
    serde_json::from_value(json!({"capabilities": {}, "workspaceFolders": folders})).unwrap()
}
