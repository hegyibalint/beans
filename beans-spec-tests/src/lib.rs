//! `beans-spec-tests` — facade-level, spec-driven behavior tests across
//! every JVM language vertical and their cross-language interactions
//! (ADR-0032).
//!
//! This crate has no library surface; the work lives in `tests/`. The
//! `src/lib.rs` exists only to give the package a target and to host
//! these notes.
//!
//! ## Layout
//!
//! - `tests/java.rs` — Java spec tests by JLS chapter, plus Java
//!   fix-behavior tests (`tests/java/`).
//! - `tests/interop.rs` — cross-language behavior (`tests/interop/`).
//! - `tests/kotlin.rs`, `tests/jvm.rs`, … — added as those verticals land.
//!
//! ## Interop directional naming
//!
//! Interop folders are named `<producer>_<consumer>`: `kotlin_java/` is a
//! Kotlin producer consumed from Java, `java_kotlin/` the reverse. The
//! asserted use site belongs to the consumer language. Full convention
//! notes live in `tests/prelude.rs`.
//!
//! ## What does *not* live here
//!
//! Vertical-local parser/model/unit tests stay next to their
//! implementation (e.g. `beans-lang-java/tests/`). This crate is for
//! product behavior exercised through the composed `beans` facade.
