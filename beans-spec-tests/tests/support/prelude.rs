//! Shared fixture entry point for every spec-test binary in this crate
//! (`java`, `interop`, and the language/`jvm` areas as they land).
//!
//! It lives under `tests/support/` rather than directly in `tests/` so
//! Cargo does not treat it as its own integration-test target; binaries
//! pull it in with `#[path = "support/prelude.rs"] mod prelude;`.
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
//! - `java_groovy/` — a Java producer consumed from Groovy, e.g. a Groovy
//!   dynamic call into a Java member.
//!
//! The use site being asserted always belongs to the *consumer*
//! language; the *producer* supplies the declaration the consumer
//! resolves across the language boundary.

use beans_test_harness::fixture::Fixture;

pub fn fixture() -> Fixture {
    Fixture::new()
}
