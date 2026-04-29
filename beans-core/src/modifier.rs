//! Source-level modifiers applied to a symbol.
//!
//! Per ADR-0001 the enum is closed and exhaustive across the JVM language
//! family. Adding a new modifier (e.g., a future Kotlin-specific keyword) is
//! a code change in this enum, not an extension point.

/// A modifier keyword written on a symbol's declaration.
///
/// `Vec<Modifier>` is the canonical form; an unmodified declaration carries
/// an empty vector rather than a sentinel. Order is the order written in
/// source — useful for round-tripping and diagnostics, not load-bearing for
/// semantic analysis.
///
/// The variants cover Java's keywords plus the keywords other JVM languages
/// share with Java. Language-specific access modifiers that have no Java
/// analogue (Scala's `private[pkg]`, Kotlin's `internal`) are *not* in this
/// enum; they live on the surrounding language node, per ADR-0004's
/// "per-language models with JVM projection" split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// `public` — accessible from anywhere (JLS §6.6).
    Public,
    /// `private` — accessible only from the enclosing top-level class (JLS §6.6).
    Private,
    /// `protected` — accessible from the package and from subclasses (JLS §6.6).
    Protected,
    /// `static` — class member, not bound to an instance (JLS §8.1.1.3).
    Static,
    /// `abstract` — declaration without an implementation (JLS §8.1.1.1).
    Abstract,
    /// `final` — class cannot be subclassed; method cannot be overridden;
    /// field cannot be reassigned (JLS §8.1.1.2, §8.4.3.3, §4.12.4).
    Final,
    /// `sealed` — class permits a fixed set of subtypes (JLS §8.1.1.2).
    Sealed,
    /// `non-sealed` — direct subtype of a sealed class that opens itself
    /// back up for further extension (JLS §8.1.1.2).
    NonSealed,
    /// `default` — interface method with a body (JLS §9.4.3).
    Default,
    /// `synchronized` — method acquires the receiver's monitor on entry
    /// (JLS §8.4.3.6).
    Synchronized,
    /// `volatile` — field reads and writes are not reordered or cached
    /// (JLS §8.3.1.4).
    Volatile,
    /// `transient` — field is skipped by Java serialization (JLS §8.3.1.3).
    Transient,
    /// `native` — method is implemented outside the JVM (JNI) (JLS §8.4.3.4).
    Native,
    /// `strictfp` — floating-point arithmetic is strictly IEEE-754
    /// (JLS §8.1.1.4). Effectively a no-op since Java 17 but still
    /// permitted in source.
    Strictfp,
}
