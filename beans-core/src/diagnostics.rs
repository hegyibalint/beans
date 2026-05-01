//! Diagnostic value type and engine plumbing.
//!
//! Per ADR-0015 (planned) and the diagnostic-rule trait backlog
//! (#015): per-language diagnostic *rules* live in
//! `crate::languages::<lang>::diagnostics`. The value type — what a
//! diagnostic looks like once produced — is universal and lives here.
//!
//! For step 6 of the graph migration the rule infrastructure is not
//! built. [`compute_diagnostics`] returns an empty list; the LSP calls
//! it eagerly on `did_open` / `did_change` / `did_save` and pushes the
//! result. The full architecture (diagnostic view nodes per file,
//! tiered subscriptions per ADR-0008) lands when subscription tiering
//! is implemented (backlog #027) — at that point the LSP swaps the
//! eager call for a registry-driven subscription firing the same
//! function. The function boundary is what makes the swap mechanical.

use crate::graph::arena::Graph;
use crate::payload::NodePayload;
use crate::primitives::Location;
use crate::registry::Registries;
use std::path::Path;

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
    /// `"unused-import"`, `"unresolved-name"`). Useful for
    /// suppression, documentation links, and structured logging.
    pub code: Option<String>,
}

/// Compute diagnostics for one file.
///
/// Today returns `Vec::new()` — the diagnostic-rule infrastructure
/// (backlog #015) is not implemented yet. Called eagerly from LSP
/// handlers (`did_open` / `did_change` / `did_save`); will be driven
/// by a registry subscription once tiered subscriptions land per
/// backlog #027.
#[allow(unused_variables)] // graph/registries/file consumed once rules land.
pub fn compute_diagnostics(
    graph: &Graph<NodePayload>,
    registries: &Registries,
    file: &Path,
) -> Vec<Diagnostic> {
    // Plumbing-only per step 6 of the graph migration. The function
    // boundary is the swap-in point: when rules land (backlog #015) and
    // subscriptions land (backlog #027), the LSP replaces its eager
    // handler-side calls with `registries.diagnostics.subscribe(file,
    // || compute_diagnostics(...))`, the rule registry runs each
    // rule's `on_node`/`on_relation` walk over `graph`, and the result
    // flows the same way it does today.
    Vec::new()
}
