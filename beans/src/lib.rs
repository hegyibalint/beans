//! beans — the facade that composes the engine, the shared JVM model,
//! and the language verticals into one whole-world surface.
//!
//! This crate owns the two closed unions that must see every language:
//!
//! - [`NodePayload`](payload::NodePayload) — the graph payload union.
//! - [`Registries`](registries::Registries) — the composed registry
//!   bag (`jvm` + one field per language vertical).
//!
//! It also hosts per-extension dispatch ([`compute_diagnostics`]) and
//! the [`Beans`] engine instance. Consumers (the LSP, CLIs, the test
//! harness) depend on this crate; verticals never depend on it.

pub mod payload;
pub mod registries;

pub mod languages {
    //! Language verticals, re-exported under their conventional names.
    #[cfg(feature = "java")]
    pub use beans_lang_java as java;

    #[cfg(feature = "kotlin")]
    pub mod kotlin;
    #[cfg(feature = "scala")]
    pub mod scala;
    #[cfg(feature = "groovy")]
    pub mod groovy;
    #[cfg(feature = "clojure")]
    pub mod clojure;
}

// Engine and shared-model re-exports keep consumer imports stable:
// `beans::Graph`, `beans::SymbolKind`, `beans::Diagnostic`, ...
pub use beans_core::{
    diagnostics, fix, graph, primitives, registry, Diagnostic, DiagnosticSeverity, Fix, Location,
    SourceEdit,
};
pub use beans_core::registry::{
    FallbackSubscription, Query, QueryResult, Registry, Subscription, Watch,
};
pub use beans_lang_jvm as jvm;
pub use beans_lang_jvm::completion;
pub use beans_lang_jvm::{
    AnnotationInstance, AnnotationValue, CompletionCandidate, CompletionCandidates, ConstantValue,
    Fqn, Modifier, PrimitiveKind, RecordComponent, SymbolKind, TypeParam, TypeRef, WildcardBound,
};
pub use payload::NodePayload;
pub use registries::Registries;

use beans_core::graph::Graph;
use std::path::Path;

/// The top-level engine instance. Per workspace, exactly one. Owns the
/// graph + registries and any future engine-wide state. Not Clone;
/// not constructed casually. Library consumers (LSP, CLI, batch tools)
/// each own one and operate it through the methods below.
pub struct Beans {
    pub graph: Graph<NodePayload>,
    pub registries: Registries,
}

impl Beans {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            registries: Registries::new(),
        }
    }
}

impl Default for Beans {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute every diagnostic that applies to `file`. Dispatches by file
/// extension to the owning vertical's rule set. Files whose extension
/// matches no enabled language feature produce an empty result, so
/// consumers can call this unconditionally on any document.
pub fn compute_diagnostics(
    graph: &Graph<NodePayload>,
    registries: &Registries,
    file: &Path,
    #[cfg(feature = "java")] java_imports: &[beans_lang_java::Import],
    roots: &[beans_core::graph::NodeId],
) -> Vec<Diagnostic> {
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        #[cfg(feature = "java")]
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
