//! Positions and ranges in source content.

use crate::{origin::Origin, ranges::ByteRange};

/// A byte range in source content identified by its origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub origin: Origin,
    pub range: ByteRange,
}

impl SourceSpan {
    pub fn new(origin: Origin, range: ByteRange) -> Self {
        Self { origin, range }
    }
}
