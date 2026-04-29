//! Cross-layer node payload union.
//!
//! Per ADR-0019 every variant the engine can store lives in one enum.
//! Per ADR-0006 a single graph holds both per-language nodes and their
//! JVM projections, so [`NodePayload`] unions the per-language payloads
//! (gated by their language feature) with the shared JVM payload.
//! Per ADR-0021 this replaces the prototype's monolithic
//! [`Symbol`](crate::Symbol).
//!
//! Variants:
//! - `Jvm` — a JVM-projection node ([`JvmNodePayload`]).
//! - `Java` — a Java-side node ([`JavaNodePayload`]). Hard-links its
//!   `Jvm` projection child per ADR-0004.
//!
//! New language variants land alongside their feature-gated module.

use crate::jvm::payload::JvmNodePayload;

#[cfg(feature = "java")]
use crate::languages::java::payload::JavaNodePayload;

/// Union of every node payload the engine can store. Variants are
/// feature-gated to match their owning language module.
#[derive(Debug, Clone, PartialEq)]
pub enum NodePayload {
    Jvm(JvmNodePayload),

    #[cfg(feature = "java")]
    Java(JavaNodePayload),
}
