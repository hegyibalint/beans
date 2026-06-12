//! beans-core — the symbolic engine.
//!
//! Storage and indexing machinery with no language and no JVM
//! knowledge:
//!
//! - [`graph`] — the typed arena: hard-link forest, generational
//!   `NodeId`, RAII `handles` (ADR-0027).
//! - [`registry`] — the `Registry<K>` primitive plus the query types
//!   (`Query`, `Subscription`, `FallbackSubscription`; ADR-0008 rev 3).
//! - [`primitives`] — source vocabulary shared by every layer
//!   (`Location`).
//! - [`diagnostics`] / [`fix`] — neutral analysis *value* types
//!   (`Diagnostic`, `Fix`). The analyses that produce them live in the
//!   vertical crates (`beans-lang-<language>`).
//!
//! The shared JVM model lives in `beans-lang-jvm`; language verticals
//! depend on it and on this crate, never on each other. The `beans`
//! facade composes the union payload and the registries bag for whole-
//! world consumers. Per ADR-0020 the LSP stays a leaf above all of it.

pub mod diagnostics;
pub mod fix;
pub mod graph;
pub mod primitives;
pub mod registry;

pub use diagnostics::{Diagnostic, DiagnosticSeverity};
pub use fix::{Fix, SourceEdit};
pub use primitives::Location;
pub use registry::{
    FallbackSubscription, Query, QueryResult, Registry, SimpleNamed, Subscription, Watch,
};
