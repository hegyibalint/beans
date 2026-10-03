//! Workspace-provided visibility, independent of semantic state and editor protocols.

use std::path::PathBuf;

use beans_core::classpath::{Classpath, Unrestricted};
use url::Url;

/// Backend contract for document URI visibility, including platform-provided virtual documents.
/// Implementations retain their own project models; consumers only borrow the resulting policy.
pub trait Workspace {
    fn classpath_for(&self, uri: &str) -> &dyn Classpath;
}

/// A workspace without a configured visibility policy.
impl Workspace for Unrestricted {
    fn classpath_for(&self, _uri: &str) -> &dyn Classpath {
        self
    }
}

/// Owns workspace bookkeeping and supplies current visibility without exposing the backend.
/// Until a workspace is configured, every stored source is visible.
///
/// ```
/// use beans_core::{query_scope::QueryScope, revision::Revision};
/// use beans_workspace::Workspaces;
///
/// fn scope<'a>(
///     workspaces: &'a Workspaces,
///     uri: &str,
///     revision: Revision,
/// ) -> QueryScope<'a> {
///     QueryScope::new(revision, workspaces.classpath_for(uri))
/// }
/// ```
#[derive(Default)]
pub struct Workspaces {
    workspace: Option<Box<dyn Workspace>>,
    registered: Vec<RegisteredWorkspace>,
}

struct RegisteredWorkspace {
    roots: Vec<PathBuf>,
    workspace: Box<dyn Workspace>,
}

impl Workspaces {
    pub fn new(workspace: impl Workspace + 'static) -> Self {
        Self {
            workspace: Some(Box::new(workspace)),
            registered: Vec::new(),
        }
    }

    /// Register a backend for absolute project/source roots. The most specific
    /// matching root wins; later registrations break ties. Other documents use
    /// the backend supplied to `new`, or unrestricted visibility by default.
    pub fn add(
        &mut self,
        roots: impl IntoIterator<Item = PathBuf>,
        workspace: impl Workspace + 'static,
    ) {
        self.registered.push(RegisteredWorkspace {
            roots: roots.into_iter().collect(),
            workspace: Box::new(workspace),
        });
    }

    pub fn classpath_for(&self, uri: &str) -> &dyn Classpath {
        if let Some(path) = Url::parse(uri)
            .ok()
            .filter(|uri| uri.scheme() == "file")
            .and_then(|uri| uri.to_file_path().ok())
        {
            let selected = self
                .registered
                .iter()
                .flat_map(|registered| {
                    registered
                        .roots
                        .iter()
                        .map(move |root| (root, &registered.workspace))
                })
                .filter(|(root, _)| path.starts_with(root))
                .max_by_key(|(root, _)| root.components().count());
            if let Some((_, workspace)) = selected {
                return workspace.classpath_for(uri);
            }
        }
        match &self.workspace {
            Some(workspace) => workspace.classpath_for(uri),
            None => &Unrestricted,
        }
    }
}

#[cfg(test)]
mod tests;
