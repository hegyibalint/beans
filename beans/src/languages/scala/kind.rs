//! Scala-specific symbol kinds.

/// A Scala-specific kind. Variants here have no direct JVM-projection
/// counterpart that preserves enough information for Scala-aware
/// dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// Scala `trait` (Scala spec §5, Traits). Distinct from a Java
    /// interface because Scala traits can carry state and concrete
    /// members, and the JVM projection differs.
    Trait,
    /// Scala `case class` (Scala spec §5.3, Case Classes) — class with
    /// auto-generated `apply`, `unapply`, `equals`, `hashCode`,
    /// `toString`, and `copy`.
    CaseClass,
    /// Scala `case object` — singleton case class (Scala spec §5.3,
    /// case-modified object definitions).
    CaseObject,
}
