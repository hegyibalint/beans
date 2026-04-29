//! Kotlin language module.
//!
//! Per ADR-0004 each JVM language has its own model. Kotlin contributes
//! kinds that don't reduce to the JVM projection without information loss:
//! `object` declarations (singleton classes), `companion object` members,
//! `data class`, and `sealed class`. They are enumerated in [`kind`] for
//! Kotlin-aware consumers; the JVM projection of a Kotlin class still uses
//! the shared [`crate::SymbolKind::Class`] tag.
//!
//! The Kotlin parser, type system, and rule set will land in later
//! migration steps; this module exists today to host [`SymbolKind`] and to
//! gate the Kotlin feature surface on `feature = "kotlin"`.

pub mod kind;

pub use kind::SymbolKind;
