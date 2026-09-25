use crate::model::ranges::ByteRange;

/// A language-provided hover result, expressed in source byte coordinates.
pub trait HoverResponse {
    fn contents(&self) -> &str;
    fn range(&self) -> ByteRange;
}
