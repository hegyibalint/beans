//! Clojure language module.
//!
//! Per ADR-0004 Clojure does not fit the class-shaped JVM model cleanly:
//! namespaces hold top-level definitions, protocols give open polymorphic
//! dispatch, multimethods carry arbitrary dispatch, and `defrecord` /
//! `deftype` produce JVM classes with extra structure. The kinds live in
//! [`kind`].
//!
//! The Clojure parser and rule set will land in later migration steps;
//! this module exists today to host [`SymbolKind`] and to gate the Clojure
//! feature surface on `feature = "clojure"`.

pub mod kind;

pub use kind::SymbolKind;
