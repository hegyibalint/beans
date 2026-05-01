//! `beans-core` — semantic graph engine, JVM model, and per-language modules.
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
//!   payload variants, registries, the typed keys.
//! - [`languages`] — per-language modules, gated by Cargo features
//!   (`java`, `kotlin`, `scala`, `groovy`, `clojure`). Each owns the
//!   rich model that doesn't reduce to the JVM projection cleanly.
//! - [`primitives`] — cross-cutting primitives (currently only
//!   [`Location`]).
//! - [`diagnostics`] — diagnostic value type and the engine plumbing
//!   that consumers run against the graph.
//! - [`payload`] / [`registries`] — the cross-layer aggregations
//!   ([`NodePayload`] union, [`Registries`] bag).
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

pub mod beans;
pub mod diagnostics;
pub mod graph;
pub mod jvm;
pub mod languages;
pub mod multi_query;
pub mod payload;
pub mod primitives;
pub mod query;
pub mod registries;
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

pub use beans::Beans;
pub use diagnostics::{compute_diagnostics, Diagnostic, DiagnosticSeverity};
pub use multi_query::{MultiQuery, MultiSubscriptionHandle, RegistryQuery};
pub use payload::NodePayload;
pub use primitives::Location;
pub use query::{all_matches, first_match, ByFqn, QueryResult, Queryable};
pub use registries::Registries;

pub use completion::{CompletionCandidate, CompletionCandidates};
