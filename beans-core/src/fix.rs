//! Fix value types — the analysis layer's "actionable half" of a
//! diagnostic.
//!
//! A [`Fix`] is a domain value, not a protocol type: it describes a
//! source change in engine vocabulary ([`Location`] spans + replacement
//! text). Consumers translate it outward — `beans-lsp` maps a `Fix` to
//! an LSP `CodeAction`/`WorkspaceEdit`; a future CLI applies the edits
//! to files directly. Per the library-first rule (ADR-0002/0020) the
//! synthesis lives here so every consumer shares one implementation.
//!
//! No fix is computed in this module; each language module's rules
//! produce `Fix` values next to the diagnostics they repair. The first
//! producer is the Java `missing-import` rule.

use crate::primitives::Location;

/// A single text replacement in one file.
///
/// `location` addresses the replaced span using the engine's standard
/// convention (zero-based lines, UTF-16 columns, exclusive end). An
/// empty span (`start == end`) is a pure insertion at that point.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceEdit {
    pub location: Location,
    pub new_text: String,
}

/// One actionable change, presented to the user under `label`.
///
/// Applying a fix means applying all of its `edits`. Edits within one
/// fix are disjoint; consumers may apply them in any order that keeps
/// earlier offsets valid (bottom-up by span is the usual choice).
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    /// Human-readable action label, e.g. `Import 'com.example.Service'`.
    pub label: String,
    pub edits: Vec<SourceEdit>,
}
