use std::{collections::BTreeSet, path::PathBuf};

use beans_core::{
    classpath::Classpath,
    location::{Location, LocationEntry},
};

use crate::{
    model::{Project, Unit},
    paths::normalize,
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

impl Classpath for SourceClasspath {
    fn contains_source(&self, resource: &Location) -> bool {
        let [LocationEntry::File(path)] = resource.entries.as_slice() else {
            return false;
        };
        if !path.is_absolute() {
            return false;
        }
        let path = normalize(path);
        self.source_roots.iter().any(|root| path.starts_with(root))
    }

    fn contains_class(&self, resource: &Location) -> bool {
        let Some(LocationEntry::File(path)) = resource.entries.first() else {
            return false;
        };
        if !path.is_absolute() {
            return false;
        }
        let path = normalize(path);
        if resource.entries.len() == 1 {
            self.artifacts.iter().any(|root| path.starts_with(root))
        } else {
            self.artifacts.contains(&path)
        }
    }
}
