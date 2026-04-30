//! Source-file source ranges used by every payload that has one.
//!
//! `Location` is the bridge between an in-memory node payload (a
//! [`crate::jvm::JvmDeclHeader::location`] or
//! [`crate::languages::java::JavaDeclHeader::location`]) and the bytes
//! on disk that produced it. Go-to-definition, hover, and diagnostics
//! all need to point back at a span in a file; this is that span.

use std::path::PathBuf;

/// A half-open source range inside a single file.
///
/// Lines and columns are zero-based to match LSP wire format
/// (`lsp_types::Position`). Columns are UTF-16 code units, again to match
/// LSP — the server does the conversion at the boundary, but `Location`
/// stores values already in LSP's coordinate system.
///
/// The end point is exclusive: a single-character span at line 3, column 5
/// has `start_col = 5` and `end_col = 6`.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    /// Absolute path to the source file. Symbols loaded from compiled
    /// artifacts (jmod, JAR) currently have no `Location`; this field is
    /// only meaningful for source-derived symbols.
    pub file: PathBuf,
    /// Zero-based start line.
    pub start_line: u32,
    /// Zero-based start column, in UTF-16 code units.
    pub start_col: u32,
    /// Zero-based end line.
    pub end_line: u32,
    /// Zero-based end column, in UTF-16 code units. Exclusive.
    pub end_col: u32,
}
