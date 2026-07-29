use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The result of an import, whichever producer made it.
///
/// Mirrors the sidecar's `dev.blnt.beans.sidecar.model.Workspace` field for
/// field, including its JSON spelling, so a JVM adapter's output and a hand
/// written descriptor land in the same type. That is the whole point of this
/// crate: producers know their tool, consumers know only this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    /// Which producer made it. Reported, never dispatched on.
    pub tool: String,
    pub units: Vec<Unit>,
}

/// One compilation scope: what it can see is its own sources, its
/// dependencies, and their outputs. Finer than a build tool's module, because
/// a Gradle project with main and test source sets is two units, and they see
/// different things.
///
/// Every path is absolute. A producer reading relative paths resolves them
/// before handing the workspace over, so consumers never need the root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub id: String,
    pub sources: Vec<Selector>,
    /// Ids of other units.
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub classpath: Vec<PathBuf>,
    #[serde(default)]
    pub jdk_home: Option<PathBuf>,
}

/// How a unit names its inputs. A `Tree` keeps matching as files appear, so
/// adding a source file needs no re-import; `Files` is the escape hatch for
/// inputs no pattern describes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Selector {
    Tree {
        base: PathBuf,
        #[serde(default)]
        includes: Vec<String>,
        #[serde(default)]
        excludes: Vec<String>,
        /// Build output rather than hand written source.
        #[serde(default)]
        generated: bool,
    },
    Files {
        files: Vec<PathBuf>,
        #[serde(default)]
        generated: bool,
    },
}
