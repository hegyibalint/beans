use std::{collections::BTreeSet, path::PathBuf};

use beans_core::{classpath::Classpath, origin::Origin};

use crate::{
    model::{Project, Unit},
    paths::{document_path, normalize},
};

#[derive(Debug, Default)]
pub(super) struct SourceClasspath {
    source_roots: BTreeSet<PathBuf>,
    artifacts: BTreeSet<PathBuf>,
}

impl SourceClasspath {
    pub(super) fn include(&mut self, unit: &Unit, project: &Project) {
        self.source_roots.extend(unit.sources.iter().cloned());
        for dependency in &unit.depends_on {
            self.source_roots
                .extend(project.units[dependency].sources.iter().cloned());
        }
        self.artifacts.extend(unit.classpath.iter().cloned());
        if let Some(home) = &unit.jdk_home {
            self.artifacts.insert(home.join("lib/modules"));
        }
    }
}

#[cfg(test)]
mod tests;

impl Classpath for SourceClasspath {
    fn contains(&self, source: &Origin) -> bool {
        match source {
            Origin::Document { uri } => document_path(uri)
                .is_some_and(|path| self.source_roots.iter().any(|root| path.starts_with(root))),
            Origin::Class { path } => {
                path.is_absolute() && {
                    let path = normalize(path);
                    self.artifacts.iter().any(|root| path.starts_with(root))
                }
            }
            Origin::JarEntry { jar_path: path, .. }
            | Origin::JmodEntry {
                jmod_path: path, ..
            }
            | Origin::JimageEntry {
                jimage_path: path, ..
            } => path.is_absolute() && self.artifacts.contains(&normalize(path)),
        }
    }
}
