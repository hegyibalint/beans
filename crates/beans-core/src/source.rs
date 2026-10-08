//! Positions and ranges in source content.

use crate::{ranges::ByteRange, resource::ResourceId};

/// A byte range in content identified by its resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub resource: ResourceId,
    pub range: ByteRange,
}

impl SourceSpan {
    pub fn new(resource: ResourceId, range: ByteRange) -> Self {
        Self { resource, range }
    }
}
