//! Diagnostic value types.
//!
//! Per ADR-0029 the layer-1 IR contains declarations and use sites; the
//! language verticals' rules read both off the graph and emit
//! [`Diagnostic`]s pointing at modifiable source positions. The engine
//! carries only the *value* vocabulary — rules, rule contexts, and the
//! per-extension dispatch live in the vertical crates and the `beans`
//! facade respectively (per ADR-0017 there is no central pipeline
//! machinery to host here).
//!
//! Per ADR-0027 lazy recomputation and caching are layer-2 concerns;
//! diagnostics are recomputed on each request until the
//! stale-while-revalidate caching of ADR-0028 lands.

use crate::primitives::Location;

/// Severity tier of a diagnostic. Mirrors the classic LSP tiering;
/// kept LSP-agnostic here so non-LSP consumers (a CLI, a build-time
/// linter) can use the same value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// One diagnostic finding. Pointed at a source [`Location`] with a
/// human-readable message, a severity, and an optional rule code.
///
/// Per ADR-0020 this type does not carry LSP wire shapes; the LSP
/// adapts it into `lsp_types::Diagnostic` at the protocol boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub location: Location,
    pub severity: DiagnosticSeverity,
    pub message: String,
    /// Stable identifier for the rule that fired (e.g.,
    /// `"unused-import"`, `"abstract-method-with-body"`). Surfaced for
    /// suppression, documentation links, and structured logging.
    pub code: Option<String>,
}
