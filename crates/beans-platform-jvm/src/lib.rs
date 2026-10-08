mod container;
mod model;

use std::path::PathBuf;

use beans_core::{resource::ResourceId, revision::Revision};
use beans_storage::RevisionedStorage;

use crate::model::class::Class;

struct JvmPlatform {
    classes: RevisionedStorage<ResourceId, Class>,
}

impl JvmPlatform {
    pub fn process(&mut self, revision: Revision, path: PathBuf) -> Result<(), String> {
        let file =
            std::fs::File::open(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let resource_id = ResourceId::file(path);
        let entries = container::jar::process(resource_id, file)?;

        for entry in entries {
            let (id, class) = entry?;
            self.classes.put(revision, id, class);
        }

        Ok(())
    }
}
