//! Cross-language interop spec tests, exercised through the composed
//! `beans` facade (ADR-0032).
//!
//! Folders under `interop/` are named `<producer>_<consumer>` — see the
//! directional-naming notes in `tests/prelude.rs`.

#[path = "support/prelude.rs"]
mod prelude;

#[path = "interop/kotlin_java/nullability_at_java_use_site.rs"]
mod nullability_at_java_use_site;
