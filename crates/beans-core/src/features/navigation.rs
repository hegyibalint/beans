use crate::{origin::Origin, ranges::ByteRange, source::SourceSpan};

/// Finds a definition using the implementation's context and request/response types.
/// Unsupported requests and positions without a navigable definition return `None`.
pub trait DefinitionProvider<C: ?Sized, Request: ?Sized, Response> {
    fn goto_definition(&self, context: &C, request: &Request) -> Option<Response>;
}

/// A position in a stored source model, expressed as a UTF-8 byte offset.
pub struct DefinitionRequest<'a> {
    pub source: &'a Origin,
    pub offset: usize,
}

/// A destination and the selected range in the source being queried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationResult {
    pub origin_range: ByteRange,
    pub target: SourceSpan,
}
