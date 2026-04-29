//! `beans-core` — semantic graph engine, JVM model, and per-language modules.
//!
//! Module layout (per ADR-0019 / ADR-0004):
//!
//! - [`graph`] — the generic graph engine (nodes, registries, hard/dynamic
//!   links). Language- and JVM-agnostic.
//! - [`jvm`] — the JVM interop layer. Modifiers, relations, signatures,
//!   structural type references, and the prototype `Symbol` shape every
//!   language's JVM projection talks to.
//! - [`languages`] — per-language modules, gated by Cargo features
//!   (`java`, `kotlin`, `scala`, `groovy`, `clojure`). Each owns the rich
//!   model that doesn't reduce to the JVM projection cleanly.
//! - [`primitives`] — cross-cutting primitives (currently only
//!   [`Location`]).
//!
//! Several top-level modules ([`completion`], [`language`], [`resolve`])
//! plus the private `symbol_id`, `symbol_kind`, and `symbol_table` modules
//! are prototype-era types being retired in subsequent migration steps.
//! They re-export through this file so existing consumers keep their
//! imports unchanged for the duration of the migration.

pub mod graph;
pub mod jvm;
pub mod languages;
pub mod primitives;

// Prototype-era modules — retired as the graph migration progresses.
pub mod completion;
pub mod language;
pub mod resolve;
mod symbol_id;
mod symbol_kind;
mod symbol_table;

// JVM model re-exports. Per ADR-0019 the JVM types live under `jvm/`;
// surfacing them at the crate root keeps consumer imports stable while the
// prototype walker and `SymbolTable` are still alive.
pub use jvm::{
    AnnotationInstance, AnnotationValue, ConstantValue, MethodParam, Modifier, PrimitiveKind,
    RecordComponent, Relation, RelationKind, Signature, Symbol, TypeParam, TypeRef, WildcardBound,
};

pub use primitives::Location;

// Prototype-era re-exports.
pub use completion::{CompletionItem, CompletionItems};
pub use language::Language;
pub use resolve::Import;
pub use symbol_id::SymbolId;
pub use symbol_kind::SymbolKind;
pub use symbol_table::SymbolTable;
