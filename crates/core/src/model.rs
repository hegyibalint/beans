use std::fmt;
use std::ops::Range;

/// A byte offset into a file's UTF-8 text. The universal internal position;
/// line/column is derived only at the edge, via a `TextFile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Offset(pub usize);

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A half-open byte range `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OffsetSpan {
    pub start: Offset,
    pub end: Offset,
}

impl OffsetSpan {
    pub fn len(&self) -> usize {
        self.end.0 - self.start.0
    }

    pub fn is_empty(&self) -> bool {
        self.start.0 == self.end.0
    }
}

impl From<Range<usize>> for OffsetSpan {
    fn from(range: Range<usize>) -> Self {
        Self {
            start: Offset(range.start),
            end: Offset(range.end),
        }
    }
}

/// A zero-based line and UTF-16 column: the coordinate system LSP speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineColumnPosition {
    pub line: u32,
    /// UTF-16 code units from the line start, per the LSP default encoding.
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineColumnSpan {
    pub start: LineColumnPosition,
    pub end: LineColumnPosition,
}
