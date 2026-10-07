mod containers;

use beans_core::{origin::Origin, revision::Revision};
use std::{collections::HashMap, path::PathBuf};

struct PlatformEntry {
    origin: Origin,
    class: Class,
}

struct JvmPlatform {
    classes: HashMap<Revision, PlatformEntry>,
}

impl JvmPlatform {
    pub fn accept(path: PathBuf) -> bool {
        containers::accept(path);
    }

    pub fn process(path: PathBuf) -> Result<PlatformEntry, String> {
        return containers::process(path);
    }

    pub fn store(self: &mut Self, revision: Revision, entry: PlatformEntry) {
        self.classes.insert(revision, entry);
    }
}
