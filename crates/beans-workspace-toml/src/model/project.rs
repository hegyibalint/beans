use std::{collections::BTreeMap, path::PathBuf};

use super::Module;

/// The resolved descriptor. All paths are absolute and lexically normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
    pub jdk_home: Option<PathBuf>,
    /// Modules keyed by descriptor ID, in deterministic order.
    pub modules: BTreeMap<String, Module>,
}
