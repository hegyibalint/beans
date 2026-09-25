use std::{
    fs,
    path::{Path, PathBuf},
};

use lsp_types::{InitializeParams, Uri};
use url::Url;

use super::{Features, OpenDocument};

impl Features {
    pub(crate) fn ingest_workspace(&mut self, params: &InitializeParams) {
        let mut roots = Vec::new();
        if let Some(folders) = &params.workspace_folders {
            roots.extend(
                folders
                    .iter()
                    .filter_map(|folder| workspace_path(&folder.uri)),
            );
        }
        if roots.is_empty() {
            // LSP 3.17: rootUri is a deprecated fallback for workspaceFolders.
            #[allow(deprecated)]
            if let Some(root) = params.root_uri.as_ref().and_then(workspace_path) {
                roots.push(root);
            }
        }

        for root in &roots {
            for path in workspace_files(root) {
                if !self.engine.accept(&path) {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(uri) = Url::from_file_path(&path) else {
                    continue;
                };
                let Ok(document_uri) = uri.as_str().parse() else {
                    continue;
                };
                self.engine.process(uri.as_str(), &text);
                self.workspace_documents.insert(
                    uri.as_str().into(),
                    OpenDocument::new(document_uri, "java".into(), 0, text),
                );
            }
        }
    }
}

fn workspace_path(workspace_uri: &Uri) -> Option<PathBuf> {
    Url::parse(workspace_uri.as_str()).ok()?.to_file_path().ok()
}

fn workspace_files(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}
