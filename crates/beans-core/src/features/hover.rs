use crate::model::{ranges::ByteRange, source::Source};

/// Supplies hover information using the implementation's context and request/response types.
/// Protocol facets can use protocol types without introducing a core dependency on them.
/// Unsupported requests and positions without hover information return `None`.
pub trait HoverProvider<C: ?Sized, Request: ?Sized, Response> {
    fn hover(&self, context: &C, request: &Request) -> Option<Response>;
}

/// A position in caller-supplied source text, which must match the queried model.
/// Offsets and ranges use UTF-8 bytes; protocol facets handle coordinate conversion.
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
