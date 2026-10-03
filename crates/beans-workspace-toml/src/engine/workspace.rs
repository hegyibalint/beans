use std::{collections::BTreeSet, path::PathBuf};

use beans_core::classpath::{Classpath, Unrestricted};
use beans_workspace::Workspace;

use super::classpath::SourceClasspath;
use crate::{model::Project, paths::document_path};

/// A resolved descriptor and its precomputed visibility policies.
#[derive(Debug)]
pub struct TomlWorkspace {
    project: Project,
    scopes: Vec<SourceScope>,
}

#[derive(Debug)]
struct SourceScope {
    root: PathBuf,
    classpath: SourceClasspath,
}

impl TomlWorkspace {
    pub(crate) fn new(project: Project) -> Self {
        let roots: BTreeSet<_> = project
            .units
            .values()
            .flat_map(|unit| unit.sources.iter().cloned())
            .collect();
        let scopes = roots
            .into_iter()
            .map(|root| {
                let mut classpath = SourceClasspath::default();
                // At each root, combine every owner, including owners of ancestor
                // roots. Selecting the deepest match then preserves shared inputs.
                for unit in project
                    .units
                    .values()
                    .filter(|unit| unit.sources.iter().any(|base| root.starts_with(base)))
                {
                    classpath.include(unit, &project);
                }
                SourceScope { root, classpath }
            })
            .collect();
        Self { project, scopes }
    }

    pub fn model(&self) -> &Project {
        &self.project
    }
}

impl Workspace for TomlWorkspace {
    /// Sources shared by units see the union of their policies. Unclaimed
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
