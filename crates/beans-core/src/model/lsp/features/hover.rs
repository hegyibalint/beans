use crate::{
    engine::Revision,
    model::{ranges::ByteRange, source::Source},
};

/// A validated position in a source snapshot. Offsets and ranges use UTF-8 bytes;
/// the LSP facet handles conversion to and from the client's position encoding.
pub struct HoverRequest<'a> {
    pub source: &'a Source,
    pub contents: &'a str,
    pub offset: usize,
}

pub struct HoverResponse {
    contents: String,
    range: ByteRange,
}

impl HoverResponse {
    pub fn new(contents: impl Into<String>, range: ByteRange) -> Self {
        Self {
            contents: contents.into(),
            range,
        }
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    pub fn range(&self) -> ByteRange {
        self.range
    }
}

/// A vertical may decline a hover when it does not own the source or position.
pub trait HoverProvider {
    fn hover(&self, revision: Revision, request: &HoverRequest<'_>) -> Option<HoverResponse>;
}
