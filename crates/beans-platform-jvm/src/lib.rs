mod container;
mod model;
mod parser;
mod query;

#[cfg(test)]
mod tests;

use std::{
    io,
    path::{Path, PathBuf},
};

use beans_core::{resource::ResourceId, revision::Revision};
use beans_storage::RevisionedStorage;

use crate::model::class::Class;

#[derive(Default)]
pub struct JvmPlatform {
    classes: RevisionedStorage<ResourceId, Class>,
}

impl JvmPlatform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_jdk_home(&mut self, revision: Revision, home: &Path) -> Result<(), String> {
        for artifact in jdk_artifacts(home)? {
            self.process(revision, artifact)?;
        }
        Ok(())
    }

    pub fn process(&mut self, revision: Revision, path: PathBuf) -> Result<(), String> {
        let mut file =
            std::fs::File::open(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let resource_id = ResourceId::file(path);

        if let Some(entries) = container::class::process(&resource_id, &mut file)? {
            return self.store(revision, entries);
        }
        if let Some(entries) = container::jar::process(&resource_id, &mut file)? {
            return self.store(revision, entries);
        }
        if let Some(entries) = container::jimage::process(&resource_id, &mut file)? {
            return self.store(revision, entries);
        }

        Err(format!("{resource_id:?}: unrecognized container format"))
    }

    fn store(
        &mut self,
        revision: Revision,
        entries: impl Iterator<Item = Result<(ResourceId, Class), String>>,
    ) -> Result<(), String> {
        for entry in entries {
            let (id, class) = entry?;
            self.classes.put(revision, id, class);
        }

        Ok(())
    }
}

/// Selects the first supported layout, including tools.jar only for legacy homes.
pub fn jdk_artifacts(home: &Path) -> Result<Vec<PathBuf>, String> {
    for (required, optional) in [
        ("lib/modules", None),
        ("jre/lib/rt.jar", Some("lib/tools.jar")),
        ("lib/rt.jar", Some("lib/tools.jar")),
    ] {
        let runtime = home.join(required);
        if !artifact_exists(&runtime)? {
            continue;
        }

        let mut artifacts = vec![runtime];
        if let Some(relative) = optional {
            let tools = home.join(relative);
            if artifact_exists(&tools)? {
                artifacts.push(tools);
            }
        }
        return Ok(artifacts);
    }
    Err(format!(
        "{}: no supported JDK runtime found",
        home.display()
    ))
}

fn artifact_exists(path: &Path) -> Result<bool, String> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(format!("{}: expected a JDK artifact file", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}
