//! JVM-shaped symbol kind tag.
//!
//! Per ADR-0019 / ADR-0004 each per-language module has its own
//! [`SymbolKind`] that names the constructs *that language* produces
//! (`kotlin::SymbolKind`,
//! `scala::SymbolKind`,
//! `clojure::SymbolKind`). This enum names the
//! constructs the JVM projection itself produces — every JVM language
//! eventually reduces to one of these.
//!
//! At the crate root, [`crate::SymbolKind`] re-exports this type so
//! existing consumers (the spec tests, the fixture, the LSP) keep
//! their `beans_core::SymbolKind` imports stable.

/// The category of a JVM-projection declaration.
///
/// `Copy` because it's a tag used heavily in `match` expressions and
/// as a filter on registry queries. `Hash` for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// A class declaration (JLS §8.1). Plain `class` keyword. Covers
    /// the JVM projection of Java/Kotlin/Scala/Groovy classes that
    /// don't have a more specific kind.
    Class,
    /// An interface declaration (JLS §9.1). Includes the JVM
    /// projection of Scala traits compiled to interfaces.
    Interface,
    /// An `enum` declaration (JLS §8.9).
    Enum,
    /// A `record` declaration (JLS §8.10). The components live as
    /// children with [`Field`](Self::Field) kind; the record's
    /// payload carries the [`RecordComponent`](crate::RecordComponent)
    /// list.
    Record,
    /// An annotation type declaration: `@interface Foo { ... }`
    /// (JLS §9.6). The declaration itself; instances of it on other
    /// declarations use [`AnnotationInstance`](crate::AnnotationInstance).
    Annotation,
    /// A method declaration (JLS §8.4). Top-level functions in
    /// Kotlin/Scala compile to static methods on a synthetic class and
    /// surface here.
    Method,
    /// A constructor declaration (JLS §8.8). Distinct from
    /// [`Method`](Self::Method) because dispatch and resolution rules
    /// differ.
    Constructor,
    /// A field declaration on a class (JLS §8.3).
    Field,
    /// A constant inside an enum body (JLS §8.9.1). Distinct from
    /// [`Field`](Self::Field) because the JVM models them as
    /// `static final` synthetics with extra structure.
    EnumConstant,
    /// A formal parameter on a method or constructor (JLS §8.4.1).
    Parameter,
    /// A package declaration (JLS §7.4). One per package; the FQN is
    /// the package's dotted name.
    Package,
}
