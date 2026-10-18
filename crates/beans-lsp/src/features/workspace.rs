use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use beans_core::location::Location;
use beans_engine::Engine;
use lsp_types::{InitializeParams, Uri};
use url::Url;

use super::{Features, OpenDocument};

impl Features {
    pub(crate) fn initialize(params: &InitializeParams) -> Result<Self, String> {
        let root = workspace_root(params)?;
        let backend = beans_workspace_toml::load(&root).map_err(|error| error.to_string())?;
        let mut engine = Engine::new(&root)?;
        engine.start()?;
        let mut features = Self {
            engine,
            client_capabilities: params.capabilities.clone(),
            open_documents: Default::default(),
            workspace_documents: Default::default(),
        };
        let mut files = BTreeSet::new();
        match backend {
            Some(backend) => {
                backend.start(&features.engine.sender())?;
                let project = backend.model();
                let source_roots: BTreeSet<_> = project
                    .units
                    .values()
                    .flat_map(|unit| unit.sources.iter().cloned())
                    .collect();
                files.extend(source_roots.iter().flat_map(|root| workspace_files(root)));
                if project
                    .units
                    .values()
                    .any(|unit| !unit.classpath.is_empty())
                {
                    log::warn!(
                        "{}: classpath ingestion is not implemented yet",
                        root.display()
                    );
                }
            }
            None => {
                files.extend(workspace_files(&root));
            }
        }
        features.ingest_files(files)?;
        Ok(features)
    }

    fn ingest_files(&mut self, files: BTreeSet<PathBuf>) -> Result<(), String> {
        let mut count = 0;
        for path in files {
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
            self.engine.process(Location::file(path), &text)?;
            self.workspace_documents.insert(
                uri.as_str().into(),
                OpenDocument::new(document_uri, "java".into(), 0, text),
            );
            count += 1;
        }
        log::debug!("Indexed {count} workspace sources");
        Ok(())
    }
}

fn workspace_root(params: &InitializeParams) -> Result<PathBuf, String> {
    // LSP 3.17 #initialize: workspaceFolders supersedes deprecated rootUri;
    // an explicit empty list is an empty workspace, not a fallback request.
    let uri = if let Some(folders) = &params.workspace_folders {
        let [folder] = folders.as_slice() else {
            return Err("expected exactly one workspace folder".into());
        };
        &folder.uri
    } else {
        #[allow(deprecated)]
        params
            .root_uri
            .as_ref()
            .ok_or("expected exactly one workspace folder")?
    };
    workspace_path(uri)
        .ok_or_else(|| format!("workspace root must be a local file URI: {}", uri.as_str()))
}

fn workspace_path(workspace_uri: &Uri) -> Option<PathBuf> {
    Url::parse(workspace_uri.as_str())
        .ok()
        .filter(|uri| uri.scheme() == "file")?
        .to_file_path()
        .ok()
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

#[cfg(test)]
mod tests;
