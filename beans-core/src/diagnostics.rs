//! Diagnostic value type, [`Rule`] trait, and the file-level dispatch.
//!
//! Per ADR-0029 the layer-1 IR contains declarations and use sites; rules
//! read both off the graph and emit [`Diagnostic`]s pointing at modifiable
//! source positions. Per ADR-0017 there is no central pipeline machinery —
//! [`compute_diagnostics`] dispatches by file extension to a per-language
//! `diagnostics::rules()` function whose return value is a plain
//! `Vec<Box<dyn Rule>>`. Adding or disabling a rule is a one-line change
//! in the owning language module.
//!
//! Per ADR-0027 lazy recomputation and caching are layer-2 concerns;
//! [`compute_diagnostics`] runs every applicable rule on each call.
//! Today the LSP invokes it eagerly from
//! `did_open`/`did_change`/`did_save`; this function boundary is the
//! swap-in point for the subscription-driven recompute that lands with
//! ADR-0028's stale-while-revalidate caching.

use crate::graph::Graph;
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
    /// `"unused-import"`, `"abstract-method-with-body"`). Surfaced for
    /// suppression, documentation links, and structured logging.
    pub code: Option<String>,
}

/// Read-only view a [`Rule`] sees during [`Rule::check`]. Borrowed from
/// the engine; lives only for the duration of the check call.
///
/// `file` is the source file the diagnostic pass is computing for.
/// Rules filter graph nodes by `Location::file` until the planned
/// `file://` root nodes land (ADR-0029 reserves the slot for the
/// modifiability axis).
///
/// `java_imports` carries the file's `import` declarations when `file`
/// is a `.java` source. Imports are not graph nodes today (see
/// ADR-0029, "Imports are still not graph nodes") so we pass them
/// through as a side channel; when `file://` root nodes land, this
/// field becomes redundant and can be removed.
pub struct RuleContext<'a> {
    pub graph: &'a Graph<NodePayload>,
    pub registries: &'a Registries,
    pub file: &'a Path,
    #[cfg(feature = "java")]
    pub java_imports: &'a [crate::languages::java::Import],
}

/// One diagnostic rule. Per ADR-0029 rules are small and single-purpose;
/// composition (running a list, merging output) lives in
/// [`compute_diagnostics`].
pub trait Rule {
    /// Stable identifier (e.g. `"abstract-method-with-body"`). Surfaced
    /// as [`Diagnostic::code`] so downstream tools can suppress, link to
    /// docs, and group findings.
    fn code(&self) -> &'static str;

    /// Run the rule against `ctx` and return any findings. The returned
    /// vector may be empty; rules return owned [`Diagnostic`] values
    /// (no shared collector) so each rule is independent and trivially
    /// reorderable.
    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic>;
}

/// Compute every diagnostic that applies to `file`. Dispatches to the
/// language module owning the file's extension; per ADR-0029 each
/// language module exposes its own `diagnostics::rules()` returning a
/// `Vec<Box<dyn Rule>>`.
///
/// Files whose extension matches no enabled language feature produce an
/// empty result. This lets the LSP call `compute_diagnostics`
/// unconditionally on any text-document URI.
///
/// Per-language file metadata that doesn't yet live on graph nodes
/// (today: Java imports, per ADR-0029's deferred `JavaImportNode`)
/// flows through extra parameters. Callers that don't have a
/// language-specific value pass an empty slice.
pub fn compute_diagnostics(
    graph: &Graph<NodePayload>,
    registries: &Registries,
    file: &Path,
    #[cfg(feature = "java")] java_imports: &[crate::languages::java::Import],
) -> Vec<Diagnostic> {
    let ctx = RuleContext {
        graph,
        registries,
        file,
        #[cfg(feature = "java")]
        java_imports,
    };
    let mut out = Vec::new();
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        #[cfg(feature = "java")]
        "java" => {
            for rule in crate::languages::java::diagnostics::rules() {
                out.extend(rule.check(&ctx));
            }
        }
        _ => {}
    }
    out
}
