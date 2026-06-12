//! Scala language module.
//!
//! Per ADR-0004 Scala contributes kinds whose JVM projection loses
//! information that Scala-aware consumers need: `trait` (carries state
//! and concrete members beyond a Java interface), `case class`, and
//! `case object`. They are enumerated in [`kind`].
//!
//! The Scala parser, type system, and rule set will land in later
//! migration steps; this module exists today to host [`SymbolKind`] and to
//! gate the Scala feature surface on `feature = "scala"`.

pub mod kind;

pub use kind::SymbolKind;
