//! Inter-symbol relationships: extends, implements, overrides, permits.
//!
//! A [`Symbol`](crate::Symbol) carries `relations: Vec<Relation>` listing
//! the typed edges it has to other symbols. Relations are the source of
//! truth for type-hierarchy queries (subtype checks, override lookup,
//! `find references` on a class).

use crate::{SymbolId, TypeRef};

/// What kind of edge a [`Relation`] represents.
///
/// The variants are exhaustive across the JVM language family per ADR-0001.
/// Each variant corresponds to a specific source construct, not a
/// generalised "is-related-to" — Java's `extends` and Scala's `extends` are
/// the same edge; Java's `implements` is its own edge because the JVM
/// distinguishes class supertypes from interface supertypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationKind {
    /// Class extends class, interface extends interface, or
    /// type parameter `extends` bound. Carries the type arguments on
    /// the target if any (e.g., `extends ArrayList<String>`)
    /// (JLS §8.1.4, §9.1.3).
    Extends,
    /// Class implements interface (Java/Kotlin) (JLS §8.1.5). Scala
    /// traits use [`Extends`](Self::Extends) instead (Scala spec §5,
    /// Traits).
    Implements,
    /// Method overrides a method in a supertype (JLS §8.4.8). The
    /// `target` points at the overridden declaration, which lives
    /// elsewhere in the table (or unresolved if the supertype is from
    /// a JAR not yet indexed).
    Overrides,
    /// `permits` clause of a sealed class (JLS §8.1.6, §8.1.1.2). The
    /// `target` is a permitted subtype.
    Permits,
    /// Clojure protocol extension — `(extend-protocol P T ...)` or
    /// `(extend-type T P ...)`. `target` is the protocol; the symbol
    /// owning the relation is the extending type. See the Clojure
    /// reference: Protocols.
    ProtocolExtends,
}

/// A typed edge from the owning symbol to another symbol.
///
/// Stored on `Symbol::relations`. Every relation has a kind and a target;
/// parameterized supertypes additionally carry their type arguments so
/// substitutions (`class Foo extends List<String>` → `Foo` is-a
/// `List<String>`) survive into resolution.
///
/// Targets are `SymbolId`s — relations are intra-table. A reference to a
/// supertype defined in a not-yet-indexed JAR is left unresolved at the
/// `SymbolTable` layer; the graph-based replacement (per ADR-0006) will
/// handle that case via dynamic links.
#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    /// What kind of relationship this is.
    pub kind: RelationKind,
    /// The symbol on the other end of the edge.
    pub target: SymbolId,
    /// Type arguments applied to the target.
    ///
    /// For `class Foo extends Producer<String>`, this is
    /// `[TypeRef::Simple { name: "String" }]`. For non-parameterized
    /// supertypes the vector is empty.
    pub type_args: Vec<TypeRef>,
}
