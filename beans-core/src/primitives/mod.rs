//! Cross-cutting primitive types that are not tied to any specific layer.
//!
//! Per ADR-0019 / ADR-0004 the model splits into a JVM layer
//! ([`crate::jvm`]) and per-language layers ([`crate::languages`]). This
//! module is reserved for the rare types that don't belong to either —
//! today only [`Location`], the source-range record every layer needs.

pub mod location;

pub use location::Location;
