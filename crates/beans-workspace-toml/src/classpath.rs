use std::{collections::BTreeSet, path::PathBuf};

use beans_core::{
    classpath::Classpath,
    resource::{ResourceId, ResourceRoot},
};

use crate::{
    model::{Module, Project},
    paths::normalize,
};

#[derive(Debug, Default)]
pub(super) struct SourceClasspath {
    source_roots: BTreeSet<PathBuf>,
    jdk_home: Option<PathBuf>,
}

impl SourceClasspath {
    pub(super) fn include(&mut self, module: &Module, project: &Project) {
        self.source_roots.extend(module.sources.iter().cloned());
        for dependency in &module.depends_on {
            self.source_roots
                .extend(project.modules[dependency].sources.iter().cloned());
        }
        self.jdk_home = project.jdk_home.clone();
    }
}

impl Classpath for SourceClasspath {
    fn contains_source(&self, resource: &ResourceId) -> bool {
        let ResourceRoot::File { path } = &resource.root else {
            return false;
        };
        if !path.is_absolute() || !resource.entries.is_empty() {
            return false;
        }
        let path = normalize(path);
        self.source_roots.iter().any(|root| path.starts_with(root))
    }

    fn contains_class(&self, resource: &ResourceId) -> bool {
        let ResourceRoot::File { path } = &resource.root else {
            return false;
        };
        if !path.is_absolute() {
            return false;
        }
        let path = normalize(path);
        self.jdk_home
            .as_ref()
            .is_some_and(|home| path.starts_with(home))
    }
}

#[cfg(test)]
mod tests;
