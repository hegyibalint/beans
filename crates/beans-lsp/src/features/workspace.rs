use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use beans_engine::Engine;

use beans_workspace_toml::{TomlWorkspace, load};
use lsp_types::{InitializeParams, Uri};
use url::Url;

use super::Features;

#[derive(Debug)]
pub(crate) enum InitializeError {
    Workspace(String),
    Storage(String),
}

impl Features {
    pub(crate) fn initialize(&mut self, params: &InitializeParams) -> Result<(), InitializeError> {
        let roots = workspace_roots(params);
        for root in &roots {
            load(root).map_err(|error| InitializeError::Workspace(error.to_string()))?;
        }
        let database = std::env::var_os("BEANS_LSP_DATABASE_PATH")
            .map(PathBuf::from)
            .or_else(|| roots.first().map(|root| root.join(".beans/index.sqlite")))
            .ok_or_else(|| {
                InitializeError::Workspace(
                    "a local workspace folder or BEANS_LSP_DATABASE_PATH is required".into(),
                )
            })?;
        let parent = database
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| {
                InitializeError::Storage("database path must include a directory".into())
            })?;
        fs::create_dir_all(parent).map_err(|error| InitializeError::Storage(error.to_string()))?;
        let mut engine = Engine::new(&database).map_err(InitializeError::Storage)?;
        engine.start().map_err(InitializeError::Storage)?;
        for root in roots {
            let workspace = TomlWorkspace::start(&root, &engine.sender())
                .map_err(InitializeError::Workspace)?;
            let sources: BTreeSet<_> = match workspace {
                Some(workspace) => workspace
                    .model()
                    .modules
                    .values()
                    .flat_map(|module| module.sources.iter().cloned())
                    .collect(),
                None => BTreeSet::from([root]),
            };
            for source in sources {
                for path in workspace_files(&source) {
                    if path
                        .extension()
                        .is_some_and(|extension| extension == "java")
                    {
                        match fs::read_to_string(&path) {
                            Ok(text) => {
                                engine
                                    .process(beans_core::resource::ResourceId::file(&path), &text)
                                    .map_err(InitializeError::Storage)?;
                            }
                            Err(error) => log::warn!("{}: {error}", path.display()),
                        }
                    }
                }
            }
        }
        self.engine = Some(engine);
        self.client_capabilities = params.capabilities.clone();
        Ok(())
    }
}

fn workspace_files(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.is_file() {
            files.push(path);
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.file_name() == ".beans" || entry.file_name() == ".git" {
                continue;
            }
            pending.push(entry.path());
        }
    }
    files.sort();
    files
}

fn workspace_roots(params: &InitializeParams) -> BTreeSet<PathBuf> {
    if let Some(folders) = &params.workspace_folders {
        folders
            .iter()
            .filter_map(|folder| workspace_path(&folder.uri))
            .collect()
    } else {
        // LSP 3.17 #initialize: workspaceFolders supersedes deprecated rootUri;
        // an explicit empty list is an empty workspace, not a fallback request.
        #[allow(deprecated)]
        params
            .root_uri
            .as_ref()
            .and_then(workspace_path)
            .into_iter()
            .collect()
    }
}

fn workspace_path(workspace_uri: &Uri) -> Option<PathBuf> {
    Url::parse(workspace_uri.as_str())
        .ok()
        .filter(|uri| uri.scheme() == "file")?
        .to_file_path()
        .ok()
}

#[cfg(test)]
mod tests;
