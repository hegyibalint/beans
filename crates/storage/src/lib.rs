//! Storage: the only stateful crate in the workspace.
//!
//! Owns all state, in two kinds that must never blur (keep them in
//! separate modules):
//! - *Durable*: the lake of source/JVM models and the indices over it,
//!   keyed by content hash. Survives restarts, shareable.
//! - *Droppable*: the memo cache of semantic queries, keyed by revision.
//!   Session-local; deleting any of it costs time, never correctness.
//!
//! Also home of the `Semantics` handle: the facade through which
//! `lang-*` rules read the world (they see nothing else of this crate).

// TODO: lake, indices, memos, Semantics.
