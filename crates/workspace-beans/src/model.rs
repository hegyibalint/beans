use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use beans_workspace::model::{Selector, Unit};
use serde::Deserialize;

/// What a bare source directory contributes. The descriptor cannot say
/// otherwise yet, because nothing hand written has needed to.
pub(crate) const JAVA_SOURCES: &str = "**/*.java";

/// The file's own shape, which is not `Workspace`. It cannot be: the model
/// promises absolute paths and a committed file cannot hold them, so a
/// translation exists either way and may as well be pleasant to write.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Descriptor {
    /// The JDK every unit compiles against unless it names its own.
    ///
    /// At the top rather than only per unit because a project has one runtime
    /// and repeating an absolute path once per unit is how it goes stale.
    #[serde(default)]
    pub(crate) jdk_home: Option<PathBuf>,
    /// Keyed by id, so a repeated unit is a TOML error rather than something
    /// we have to notice. Ordered, so the same file always imports the same.
    #[serde(default)]
    pub(crate) unit: BTreeMap<String, DescriptorUnit>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DescriptorUnit {
    #[serde(default)]
    sources: Vec<PathBuf>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    classpath: Vec<PathBuf>,
    /// A JDK of this unit's own, for a project whose units do not share one.
    #[serde(default)]
    jdk_home: Option<PathBuf>,
}

impl DescriptorUnit {
    /// `default_jdk_home` is whatever the descriptor said at the top, which a
    /// unit may replace but not remove: a unit with no JDK at all is what a
    /// descriptor naming none anywhere gives.
    pub(crate) fn resolve(self, id: String, root: &Path, default_jdk_home: Option<&Path>) -> Unit {
        Unit {
            id,
            sources: self
                .sources
                .into_iter()
                .map(|base| Selector::Tree {
                    base: root.join(base),
                    includes: vec![JAVA_SOURCES.to_string()],
                    excludes: Vec::new(),
                    generated: false,
                })
                .collect(),
            depends_on: self.depends_on,
            classpath: self
                .classpath
                .into_iter()
                .map(|path| root.join(path))
                .collect(),
            // A JDK lives outside the project, so this is the one path that is
            // normally already absolute. `join` leaves such a path alone, so
            // spelling it relative still works and needs no separate rule.
            jdk_home: self
                .jdk_home
                .or_else(|| default_jdk_home.map(Path::to_path_buf))
                .map(|home| root.join(home)),
        }
    }
}
