//! Shared fixture entry point for every spec-test binary in this crate
//! (`java`, `interop`, and the language/`jvm` areas as they land).
//!
//! ## Interop directional naming
//!
//! Cross-language tests live under `tests/interop/<producer>_<consumer>/`.
//! The folder name reads **producer first, consumer second**:
//!
//! - `kotlin_java/` — a Kotlin **producer** (declares the symbol) seen
//!   from a Java **consumer** (the use site under test). Example:
//!   Kotlin nullability surfaced at a Java call site.
//! - `java_kotlin/` — the reverse: a Java producer consumed from Kotlin,
//!   e.g. a Java override of a Kotlin declaration.
//! - `groovy_java/` — a Groovy consumer calling into Java, etc.
//!
//! The use site being asserted always belongs to the *consumer*
//! language; the *producer* supplies the declaration the consumer
//! resolves across the language boundary.

use beans_test_harness::fixture::Fixture;

pub fn fixture() -> Fixture {
    Fixture::new()
}
