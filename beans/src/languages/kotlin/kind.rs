//! Kotlin-specific symbol kinds.
//!
//! These variants exist because the corresponding Kotlin construct does not
//! reduce to a JVM-shaped [`crate::SymbolKind`] without information loss —
//! a Kotlin `object` is more than `Class`, a `data class` carries
//! auto-generated members the JVM projection cannot infer back, and so on.
//! Within-Kotlin consumers dispatch on these directly; cross-language
//! consumers see the JVM projection's [`crate::SymbolKind::Class`] instead.

/// A Kotlin-specific kind. Variants here have no direct JVM-projection
/// counterpart that preserves enough information for Kotlin-aware
/// dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// Kotlin `object` declaration — singleton class (Kotlin spec
    /// §4.1.7). The JVM projection is a final class with a static
    /// `INSTANCE` field.
    Object,
    /// Kotlin `companion object` — the static-side namespace of a class
    /// (Kotlin spec §4.1.7). JVM-projected to a nested class plus a
    /// static field on the owner.
    CompanionObject,
    /// Kotlin `data class` (Kotlin spec §4.1.2) — class with
    /// auto-generated `equals`, `hashCode`, `toString`, `copy`, and
    /// `componentN`.
    DataClass,
    /// Kotlin `sealed class` (Kotlin spec §5.1.2) — Kotlin's
    /// sealed-hierarchy form. Predates Java's sealed classes and has
    /// slightly different rules (sealed subclasses must live in the
    /// same package and module).
    SealedClass,
}
