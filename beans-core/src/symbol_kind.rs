//! Tag enum identifying what a [`Symbol`](crate::Symbol) declares.
//!
//! `SymbolKind` is the one piece of the symbol model where ADR-0001
//! ("cohesive, not extensible") shows most explicitly. The variants are
//! organised by which language family contributes them: a shared core
//! covering everything Java models, then per-language additions for Kotlin,
//! Scala, and Clojure constructs that have no direct Java analogue.
//!
//! There is no plugin path. A new JVM language is a code change to this
//! enum (and to any `match` that consumes it). A future sixth language
//! that fits the JVM model would add its own variant block here; one that
//! does not fit the JVM model is out of scope by design.

/// The category of a symbol — class, method, field, namespace, etc.
///
/// `SymbolKind` is `Copy` because it is a tag, used heavily in `match`
/// expressions and as a filter on symbol-table queries (e.g., "give me all
/// methods named `process`"). It is `Hash` for the same reason.
///
/// The variants are grouped by language origin. The shared JVM block
/// covers constructs every JVM language can express in some form (every
/// language has classes, methods, fields). The per-language blocks add
/// constructs that exist in only one language and don't naturally fold
/// into a shared variant — `SealedClass` is its own variant rather than
/// `Class { sealed: true }` because matchers downstream want to dispatch
/// on it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    // ---- Shared JVM ----
    /// A class declaration (JLS §8.1). Covers Java/Kotlin/Scala/Groovy
    /// classes that are not otherwise distinguished (not records, not
    /// data classes, etc.). The plain `class` keyword.
    Class,
    /// An interface declaration (JLS §9.1). Java interfaces, Kotlin
    /// interfaces, Scala traits compiled to interfaces.
    Interface,
    /// An `enum` declaration (JLS §8.9). Java/Kotlin enums; Scala 3
    /// `enum` declarations (Scala spec §5, Enum Definitions).
    Enum,
    /// A `record` declaration (JLS §8.10). The components live as
    /// children with [`Field`](Self::Field) kind; the record's signature
    /// carries the [`RecordComponent`](crate::RecordComponent) list.
    Record,
    /// An annotation type declaration: `@interface Foo { ... }`
    /// (JLS §9.6). The declaration itself; instances of it on other
    /// symbols use [`AnnotationInstance`](crate::AnnotationInstance).
    Annotation,
    /// A method declaration (JLS §8.4). Top-level functions in
    /// Kotlin/Scala compile to static methods on a synthetic class and
    /// surface here too (Kotlin spec §4.2; Scala spec §4, Function
    /// Declarations and Definitions).
    Method,
    /// A constructor declaration (JLS §8.8). Distinct from
    /// [`Method`](Self::Method) because dispatch and resolution rules
    /// differ.
    Constructor,
    /// A field declaration on a class (JLS §8.3).
    Field,
    /// A constant inside an enum body: `enum E { A, B }` produces two
    /// `EnumConstant` symbols (JLS §8.9.1). Distinct from
    /// [`Field`](Self::Field) because the JVM models them as
    /// `static final` synthetics.
    EnumConstant,
    /// A formal parameter on a method or constructor (JLS §8.4.1).
    Parameter,
    /// A package declaration (JLS §7.4). One symbol per package; the
    /// `fqn` is the package's dotted name.
    Package,

    // ---- Kotlin-specific ----
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

    // ---- Scala-specific ----
    /// Scala `trait` (Scala spec §5, Traits). Distinct from
    /// [`Interface`](Self::Interface) because Scala traits can carry
    /// state and concrete members, and the JVM projection differs.
    Trait,
    /// Scala `case class` (Scala spec §5.3, Case Classes) — class with
    /// auto-generated `apply`, `unapply`, `equals`, `hashCode`,
    /// `toString`, and `copy`.
    CaseClass,
    /// Scala `case object` — singleton case class (Scala spec §5.3,
    /// case-modified object definitions).
    CaseObject,

    // ---- Clojure-specific ----
    /// Clojure `(ns my.thing ...)` declaration. The closest analogue to a
    /// Java package, but Clojure namespaces also own `def`s directly.
    /// See the Clojure reference: Namespaces.
    Namespace,
    /// Clojure `(defn name ...)` — top-level function bound in a namespace.
    /// JVM-projected to a static method on the namespace's compiled class.
    /// See the Clojure reference: Special Forms (`def`, `fn`).
    Function,
    /// Clojure `(defprotocol P ...)` — open polymorphic dispatch over a
    /// type. JVM-projected to an interface plus a dispatch table. See
    /// the Clojure reference: Protocols.
    Protocol,
    /// Clojure `(defmulti name dispatch-fn)` — multi-method with arbitrary
    /// dispatch. No direct JVM construct; modelled separately. See the
    /// Clojure reference: Multimethods.
    Multimethod,
    /// Clojure `(defrecord Name [...])` — generates a Java class with
    /// typed fields, plus protocol implementations. See the Clojure
    /// reference: Datatypes.
    Defrecord,
    /// Clojure `(deftype Name [...])` — like [`Defrecord`](Self::Defrecord)
    /// but without the map-like behaviour layered on. See the Clojure
    /// reference: Datatypes.
    Deftype,
}
