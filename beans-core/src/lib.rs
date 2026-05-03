//! `beans-core` — semantic graph engine, JVM model, and per-language modules.
//!
//! The top-level type is [`Beans`] — per workspace, exactly one. It owns
//! the graph and the registries; consumers (LSP, future CLI, batch
//! tools) construct one and operate the engine through it.
//!
//! Module layout (per ADR-0019 / ADR-0004):
//!
//! - [`graph`] — the generic graph engine (nodes, hard links, dynamic-link
//!   edges, cache state). Pure structure and lifecycle; no indexing.
//! - [`registry`] — typed-key index over `NodeId`s with subscription /
//!   notification (ADR-0008/0013/0014/0015). Consumes `graph` for
//!   `NodeId` and the `NodeHandle` marker; graph does not depend on
//!   registry.
//! - [`jvm`] — the JVM interop layer. Modifiers, type references, JVM
//!   payload variants, the typed keys.
//! - [`languages`] — per-language modules, gated by Cargo features
//!   (`java`, `kotlin`, `scala`, `groovy`, `clojure`). Each owns the
//!   rich model that doesn't reduce to the JVM projection cleanly.
//! - [`primitives`] — cross-cutting primitives (currently only
//!   [`Location`]).
//! - [`diagnostics`] — diagnostic value type and the engine plumbing
//!   that consumers run against the graph.
//! - [`payload`] — the cross-layer [`NodePayload`] union; the
//!   [`Registries`] bag and the query types ([`Query`],
//!   [`Subscription`], [`FallbackSubscription`], [`QueryResult`])
//!   live under [`registry`].
//! - [`completion`] — neutral completion result types
//!   ([`CompletionCandidate`], [`CompletionCandidates`]). The
//!   LSP-shaped item lives in `beans-lsp`; per ADR-0020 the core
//!   stays free of LSP-protocol shapes.
//!
//! At the crate root the JVM types are re-exported (`Modifier`,
//! `SymbolKind`, `TypeRef`, ...) so consumers write
//! `beans_core::SymbolKind` and get the JVM-shaped enum. Per-language
//! variants (`languages::kotlin::SymbolKind` etc.) are reachable via
//! their owning module.

pub mod diagnostics;
pub mod graph;
pub mod jvm;
pub mod languages;
pub mod payload;
pub mod primitives;
pub mod registry;

// Neutral completion result types. Per ADR-0020 the LSP-shaped
// `CompletionItem` lives in `beans-lsp`; the core just names what
// completed *at*.
pub mod completion;

// JVM model re-exports. Per ADR-0019 the JVM types live under `jvm/`;
// surfacing them at the crate root keeps consumer imports stable.
pub use jvm::{
    AnnotationInstance, AnnotationValue, ConstantValue, Modifier, PrimitiveKind, RecordComponent,
    SymbolKind, TypeParam, TypeRef, WildcardBound,
};

pub use diagnostics::{compute_diagnostics, Diagnostic, DiagnosticSeverity};
pub use payload::NodePayload;
pub use primitives::Location;
pub use registry::{
    FallbackSubscription, Query, QueryResult, Registries, Subscription, Watch,
};

pub use completion::{CompletionCandidate, CompletionCandidates};

use crate::graph::Graph;

/// The top-level engine instance. Per workspace, exactly one. Owns the
/// graph + registries and any future engine-wide state. Not Clone;
/// not constructed casually. Library consumers (LSP, CLI, batch tools)
/// each own one and operate it through the methods below.
///
/// Today the struct is a thin wrapper. Engine-wide state that doesn't
/// belong to either the graph or any single registry (workspace root,
/// file → roots map, future generation counter, future snapshot
/// metadata) lands here as the runtime grows.
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
