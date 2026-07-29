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
}

impl DescriptorUnit {
    pub(crate) fn resolve(self, id: String, root: &Path) -> Unit {
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
            classpath: Vec::new(),
            jdk_home: None,
        }
    }
}
