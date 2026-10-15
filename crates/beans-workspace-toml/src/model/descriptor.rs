use std::{collections::BTreeMap, path::PathBuf};

use serde::Deserialize;

/// The committed file's shape; unlike `Project`, its paths may be relative.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Descriptor {
    pub jdk_home: Option<PathBuf>,
    pub module: BTreeMap<String, DescriptorModule>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct DescriptorModule {
    pub sources: Vec<PathBuf>,
    pub depends_on: Vec<String>,
}
