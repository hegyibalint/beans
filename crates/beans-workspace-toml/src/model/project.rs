use std::{collections::BTreeMap, path::PathBuf};

use super::Unit;

/// The resolved descriptor. All paths are absolute and lexically normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
    /// Units keyed by descriptor ID, in deterministic order.
    pub units: BTreeMap<String, Unit>,
}
