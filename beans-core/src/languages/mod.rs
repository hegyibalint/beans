//! Per-language modules.
//!
//! Per ADR-0019 each supported JVM language is a feature-gated submodule of
//! `beans-core`, not a separate crate. Per ADR-0004 each owns its rich
//! language model — the kinds, modifiers, and shapes that don't reduce to
//! the [`crate::jvm`] projection without information loss. Within-language
//! work consults the rich model; cross-language work goes through the JVM
//! projection.
//!
//! Modules are gated by the matching Cargo feature
//! (`java`, `kotlin`, `scala`, `groovy`, `clojure`). All five are enabled by
//! default; consumers that only need bytecode analysis disable them.

#[cfg(feature = "java")]
pub mod java;

#[cfg(feature = "kotlin")]
pub mod kotlin;

#[cfg(feature = "scala")]
pub mod scala;

#[cfg(feature = "groovy")]
pub mod groovy;

#[cfg(feature = "clojure")]
pub mod clojure;
