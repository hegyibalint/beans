//! A `beans.toml` workspace backend, independent of language state and editor protocols.
//!
//! ```
//! use std::path::Path;
//! use beans_workspace::Workspaces;
//! use beans_workspace_toml::parse;
//!
//! let backend = parse(
//!     "[module.app]\nsources = [\"src\"]\n",
//!     Path::new("/project"),
//! )?;
//! let workspaces = Workspaces::new(backend);
//! let classpath = workspaces.classpath_for("file:///project/src/Example.java");
//! # Ok::<(), beans_workspace_toml::ParseError>(())
//! ```

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use beans_core::{
    classpath::{Classpath, Unrestricted},
    events::{Event, JavaHomeFound, ResourceFound},
    resource::ResourceId,
};
use beans_workspace::Workspace;
use crossbeam_channel::Sender;

use crate::{classpath::SourceClasspath, model::Project, paths::document_path};

pub mod model;
pub mod parser;

mod classpath;
mod jdk;
mod paths;

#[cfg(test)]
mod tests;

pub use parser::{DESCRIPTOR, LoadError, ParseError, load, parse};

/// A resolved descriptor and its precomputed visibility policies.
#[derive(Debug)]
pub struct TomlWorkspace {
    project: Project,
    scopes: Vec<SourceScope>,
}

impl TomlWorkspace {
    /// Loads the descriptor and reports its optional JDK after artifact discovery succeeds.
    pub fn start(root: &Path, events: &Sender<Event>) -> Result<Option<Self>, String> {
        let Some(workspace) = load(root).map_err(|error| error.to_string())? else {
            return Ok(None);
        };
        workspace.discover_jdk(events)?;
        Ok(Some(workspace))
    }

    pub(crate) fn new(project: Project) -> Self {
        let roots: BTreeSet<_> = project
            .modules
            .values()
            .flat_map(|module| module.sources.iter().cloned())
            .collect();
        let scopes = roots
            .into_iter()
            .map(|root| {
                let mut classpath = SourceClasspath::default();
                // At each root, combine every owner, including owners of ancestor
                // roots. Selecting the deepest match then preserves shared inputs.
                for module in project
                    .modules
                    .values()
                    .filter(|module| module.sources.iter().any(|base| root.starts_with(base)))
                {
                    classpath.include(module, &project);
                }
                SourceScope { root, classpath }
            })
            .collect();
        Self { project, scopes }
    }

    pub fn model(&self) -> &Project {
        &self.project
    }

    fn discover_jdk(&self, events: &Sender<Event>) -> Result<(), String> {
        let Some(home) = &self.project.jdk_home else {
            return Ok(());
        };
        let artifacts = match jdk::artifacts(home) {
            Ok(artifacts) => artifacts,
            Err(error) => {
                log::warn!("{error}");
                return Ok(());
            }
        };
        events
            .send(Event::JavaHomeFound(JavaHomeFound {
                home: home.clone(),
                version: None,
            }))
            .map_err(|error| error.to_string())?;
        for artifact in artifacts {
            events
                .send(Event::ResourceFound(ResourceFound {
                    resource: ResourceId::file(artifact),
                }))
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

impl Workspace for TomlWorkspace {
    /// Sources shared by modules see the union of their policies. Unclaimed
    /// files, non-file URIs, and virtual documents remain unrestricted.
    fn classpath_for(&self, uri: &str) -> &dyn Classpath {
        let Some(path) = document_path(uri) else {
            return &Unrestricted;
        };
        match self
            .scopes
            .iter()
            .filter(|scope| path.starts_with(&scope.root))
            .max_by_key(|scope| scope.root.components().count())
        {
            Some(scope) => &scope.classpath,
            None => &Unrestricted,
        }
    }
}

#[derive(Debug)]
struct SourceScope {
    root: PathBuf,
    classpath: SourceClasspath,
}
