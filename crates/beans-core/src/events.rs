use std::path::PathBuf;

use crate::{resource::ResourceId, revision::Revision};

#[derive(Debug, Clone)]
pub enum Event {
    JavaHomeFound(JavaHomeFound),
    ResourceFound(ResourceFound),
    ResourceProcessed(ResourceProcessed),
}

#[derive(Debug, Clone)]
pub struct JavaHomeFound {
    pub home: PathBuf,
    pub version: Option<String>,
}

/// Discovery does not imply that this resource is supported, loaded, or valid.
#[derive(Debug, Clone)]
pub struct ResourceFound {
    pub resource: ResourceId,
}

/// Reports a completed processing attempt. A revision is present only after publication commits.
#[derive(Debug, Clone)]
pub struct ResourceProcessed {
    pub resource: ResourceId,
    pub result: Result<Option<Revision>, String>,
}
