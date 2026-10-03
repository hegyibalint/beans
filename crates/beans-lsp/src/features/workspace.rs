use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use beans_core::model::classpath::Unrestricted;
use beans_workspace::Workspaces;
use beans_workspace_toml::{LoadError, load};
use lsp_types::{InitializeParams, Uri};
use url::Url;

use super::{Features, OpenDocument};

impl Features {
    pub(crate) fn initialize(&mut self, params: &InitializeParams) -> Result<(), LoadError> {
        // Validate every descriptor before changing semantic or workspace state.
        let imports: Vec<_> = workspace_roots(params)
            .into_iter()
            .map(|root| load(&root).map(|backend| (root, backend)))
            .collect::<Result<_, _>>()?;

        let folder_roots: Vec<_> = imports.iter().map(|(root, _)| root.clone()).collect();
        self.client_capabilities = params.capabilities.clone();
        let mut workspaces = Workspaces::default();
        let mut files = BTreeSet::new();
        for (root, backend) in imports {
            match backend {
                Some(backend) => {
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
                        .any(|unit| !unit.classpath.is_empty() || unit.jdk_home.is_some())
                    {
                        log::warn!(
                            "{}: classpath and JDK ingestion is not implemented yet",
                            root.display()
                        );
                    }
                    // Explicit folders select their own backend. Additional registrations
                    // are only needed for declared sources outside every workspace folder.
                    let external_roots = source_roots.into_iter().filter(|source| {
                        !folder_roots.iter().any(|folder| source.starts_with(folder))
                    });
                    workspaces.add(
                        std::iter::once(project.root.clone()).chain(external_roots),
                        backend,
                    );
                }
                None => {
                    files.extend(workspace_files(&root));
                    workspaces.add([root], Unrestricted);
                }
            }
        }
        self.workspaces = workspaces;
        self.ingest_files(files);
        Ok(())
    }

    fn ingest_files(&mut self, files: BTreeSet<PathBuf>) {
        let mut count = 0;
        for path in files {
            if !self.languages.accept(&path) {
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
            self.languages.process(uri.as_str(), &text);
            self.workspace_documents.insert(
                uri.as_str().into(),
                OpenDocument::new(document_uri, "java".into(), 0, text),
            );
            count += 1;
        }
        log::debug!("Indexed {count} workspace sources");
    }
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
