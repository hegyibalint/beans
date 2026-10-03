use std::{collections::BTreeMap, path::PathBuf};

use serde::Deserialize;

/// The committed file's shape; unlike `Project`, its paths may be relative.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Descriptor {
    pub jdk_home: Option<PathBuf>,
    pub unit: BTreeMap<String, DescriptorUnit>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct DescriptorUnit {
    pub sources: Vec<PathBuf>,
    pub depends_on: Vec<String>,
    pub classpath: Vec<PathBuf>,
    pub jdk_home: Option<PathBuf>,
}
