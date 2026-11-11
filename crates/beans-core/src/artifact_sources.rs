use serde::{Deserialize, Serialize};

use crate::{location::Location, resource_filter::ResourceFilter};

/// Associates each listed artifact with source resources representing it.
#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactSources {
    /// Logical artifact roots, such as JARs or class directories; they need not exist.
    pub artifacts: Vec<Location>,
    /// Source files, directories, or archives associated with these artifacts.
    /// This is not the list of dependencies used to compile them.
    pub sources: Vec<ResourceFilter>,
}
