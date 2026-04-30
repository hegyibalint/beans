//! `beans-core` — semantic graph engine, JVM model, and per-language modules.
//!
//! Module layout (per ADR-0019 / ADR-0004):
//!
//! - [`graph`] — the generic graph engine (nodes, registries, hard /
//!   dynamic links). Language- and JVM-agnostic.
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
//! - [`completion`] — the LSP-shaped completion result type. Per
//!   backlog #025 step 8 of the migration moves this into `beans-lsp`
//!   and replaces it with a neutral `CompletionCandidate` here.
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
pub mod registries;

// LSP-shaped completion result type. Per backlog #025 step 8 splits
// this into a neutral `CompletionCandidate` in `beans-core` and an
// LSP-shaped `CompletionItem` in `beans-lsp`.
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
pub use registries::Registries;

pub use completion::{CompletionItem, CompletionItems};
