//! Positions and ranges in source content.

use crate::{location::Location, ranges::ByteRange};

/// A byte range in content identified by its resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub resource: Location,
    pub range: ByteRange,
}

impl SourceSpan {
    pub fn new(resource: Location, range: ByteRange) -> Self {
        Self { resource, range }
    }
}
