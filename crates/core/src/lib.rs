//! Base abstractions shared across the workspace: identifiers and value
//! types with no crate-specific behavior.

/// The global revision counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Revision(u64);

/// Identifies uniquely a file across the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

pub struct VirtualFile {
    pub uri: String,
    pub content: String,
}
