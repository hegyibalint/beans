//! beans — the facade that composes the engine, the shared JVM model,
//! and the language verticals into one whole-world surface.
//!
//! This crate owns the two closed unions that must see every language:
//!
//! - [`NodePayload`](payload::NodePayload) — the graph payload union.
//! - [`Registries`](registries::Registries) — the composed registry
//!   bag (`jvm` + one field per language vertical).
//!
//! On top of that storage substrate it hosts the product-facing facade:
//!
//! - [`Store`] — the storage aggregate (graph + registries + interner).
//!   Consumers that want raw engine access reach for this.
//! - [`Workspace`](workspace::Workspace) — the orchestration facade. It
//!   owns workspace policy (artifact classification, parser dispatch,
//!   per-file indexing context) and exposes the consumer-level API
//!   (`update_file`, `remove_file`, `index_workspace`, `definition_at`,
//!   `references_at`, `hover_at`, `document_symbols`, `diagnostics`,
//!   `quick_fixes_at`). The LSP and any future CLI drive
//!   indexing and resolution through it rather than reimplementing the
//!   mechanics.
//!
//! It also hosts per-extension dispatch ([`compute_diagnostics`]).
//! Consumers (the LSP, CLIs, the test harness) depend on this crate;
//! verticals never depend on it.

pub mod payload;
pub mod registries;
pub mod view;
pub mod workspace;

pub mod languages {
    //! Language verticals, re-exported under their conventional names.
    //! One re-export per vertical crate that exists today; new languages
    //! join here as their `beans-lang-*` crates land.
    pub use beans_lang_java as java;
}

// Engine and shared-model re-exports keep consumer imports stable:
// `beans::Graph`, `beans::SymbolKind`, `beans::Diagnostic`, ...
pub use beans_core::registry::{
    FallbackSubscription, Query, QueryResult, Registry, Subscription, Watch,
};
pub use beans_core::{
    Diagnostic, DiagnosticSeverity, Fix, Interner, Location, SourceEdit, diagnostics, fix, graph,
    primitives, registry,
};
pub use beans_lang_jvm as jvm;
pub use beans_lang_jvm::completion;
pub use beans_lang_jvm::{
    AnnotationInstance, AnnotationValue, CompletionCandidate, CompletionCandidates, ConstantValue,
    Fqn, Modifier, PrimitiveKind, RecordComponent, SymbolKind, TypeParam, TypeRef, WildcardBound,
};
pub use payload::NodePayload;
pub use registries::Registries;
pub use view::{DocSymbol, PayloadView, payload_view};
pub use workspace::Workspace;

use beans_core::graph::Graph;
use std::path::Path;

/// The storage aggregate: graph + registries + interner. This is the
/// raw engine substrate — no workspace policy, no per-file bookkeeping.
/// [`Workspace`] owns one and layers orchestration on top; consumers
/// that want low-level access (benchmarks, the test harness) can hold a
/// `Store` directly. Not `Clone`; per workspace there is exactly one.
pub struct Store {
    pub graph: Graph<NodePayload>,
    pub registries: Registries,
    /// Workspace string interner (backlog #037). Parsed plans are
    /// interned at the integrate boundary; see
    /// `ParsedJavaFile::intern`.
    pub interner: Interner,
}

impl Store {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            registries: Registries::new(),
            interner: Interner::new(),
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute every diagnostic that applies to `file`. Dispatches by file
/// extension to the owning vertical's rule set. Files whose extension
/// matches no supported language produce an empty result, so consumers
/// can call this unconditionally on any document.
pub fn compute_diagnostics(
    graph: &Graph<NodePayload>,
    registries: &Registries,
    file: &Path,
    java_imports: &[beans_lang_java::Import],
    roots: &[beans_core::graph::NodeId],
) -> Vec<Diagnostic> {
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "java" => beans_lang_java::diagnostics::check_file(
            graph,
            &registries.java,
            &registries.jvm,
            file,
            java_imports,
            roots,
        ),
        _ => Vec::new(),
    }
}
