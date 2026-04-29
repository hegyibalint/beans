# ADR-0004: Per-language models with a shared JVM projection

## Status

Accepted

## Context

The original prototype used a single `SymbolTable` with a unified `Symbol`
type and a `SymbolKind` enum that enumerated every variant any language
might need. This is a tempting design: one model, one index, one set of
queries. It worked well enough for Java alone.

It does not survive contact with the rest of the JVM family. When we
modeled what each language actually needs to express, the gap was obvious:

- **Java** maps to a universal model with maybe ~75% fidelity. Annotations,
  generic bounds, and varargs need careful treatment but mostly fit.
- **Kotlin** maps at maybe ~40%. Nullable types are first-class. So are
  data classes, sealed hierarchies, companion objects, extension
  functions, inline/value classes, and a different visibility model.
  Squeezing all of that into Java-shaped types loses information.
- **Scala** maps at maybe ~25%. Higher-kinded types, implicits, traits
  with multiple inheritance, given/using, opaque types, path-dependent
  types. A universal model for Scala-as-Java erases most of what makes
  Scala Scala.
- **Clojure** doesn't fit at all without violence — namespaces instead of
  classes, dynamic typing, multimethods, protocols, macros.

A universal model that tries to express all of this is either lowest-
common-denominator (loses information for every language) or maximalist
(an enum with 200 variants and no clear semantics for any of them). Both
are bad.

At the same time, the entire premise of beans (ADR-0001) is that JVM
languages share a runtime and need to navigate across each other. A
Kotlin file calls a Java method. A Scala class extends a Java interface.
A Groovy test instantiates a Java class. Cross-language navigation
demands some shared vocabulary.

## Decision

**Each language has its own rich model. Cross-language interop goes
through a shared JVM projection.**

Concretely:

- Each `beans-lang-<lang>` crate owns its language model. The Kotlin model
  represents nullability, data classes, companion objects natively. The
  Scala model represents higher-kinded types, traits, given/using
  natively. The Clojure model is namespace-and-function shaped, not
  class-shaped. No language is forced into a shape it does not fit.
- A shared **JVM layer** defines a JVM-flavored symbol type — classes,
  methods, fields, packages, the things that exist at the bytecode
  level. Every language node projects to a JVM node.
- The JVM projection carries a small set of **promoted enrichments**:
  universal information that cross-language consumers benefit from
  (nullability is the first one — Kotlin promotes it, Java consumes it
  via `@Nullable` annotations). Promotion is explicit and minimal.
- Within-language operations (Kotlin completion in a Kotlin file, Scala
  type inference in a Scala file) use the rich language model.
- Cross-language operations (Kotlin code calling a Java method, finding
  references to a Scala trait from Java) use the JVM projection.

This is structurally similar to JetBrains' UAST, but built in for our
five languages from day one rather than retrofitted.

## Consequences

**Positive.**

- Each language gets a model that reflects what its users care about.
  Kotlin nullability is not a hack on top of a Java-shaped type. Scala
  trait composition is not encoded as a Java interface with extra notes.
- Cross-language navigation works because the JVM projection is the
  shared vocabulary. The JVM is the only thing all five languages
  genuinely share, so it is the right place to draw the line.
- Adding a language quirk does not destabilize the universal model. A
  new Kotlin language feature lands in the Kotlin model and either
  promotes to the JVM layer (if cross-language consumers care) or
  doesn't (if it's Kotlin-internal).
- The JVM projection is small. It's the things bytecode actually
  represents. This keeps the shared vocabulary tractable.

**Negative.**

- We maintain N+1 models (one per language plus the JVM projection)
  rather than one. More code, more types, more places to update when a
  shared concept changes.
- Promotion decisions ("is nullability promoted to the JVM layer?
  Are Scala implicit conversions?") become design decisions in their
  own right. Each promoted enrichment is a commitment.
- Cross-language operations have to traverse the projection, not the
  rich model. A reference from Kotlin to Java goes Kotlin model →
  Kotlin's JVM projection → Java's JVM projection → Java model.
  More layers, more indirection.
- Deduplicating across languages is harder. A Kotlin class and its
  generated `MyClass$Companion` Java view are the same thing at the
  JVM level. The model has to make this explicit.

## Alternatives considered

**Single universal model (the previous approach).** One Symbol type with
a SymbolKind enum that covers all languages. Rejected because, as
described above, it produces a model that is mediocre for every language.
Worse, it forces every language change to negotiate the universal model,
which scales poorly. The current `beans-core::SymbolTable` is this
design; ADR-0003 lets us replace it.

**Completely separate per-language models with no JVM bridge.** Each
language has its own model and its own index, with no shared vocabulary.
Rejected because it makes cross-language navigation either impossible
or ad hoc (every language has to know how to talk to every other,
N×(N-1) integration points instead of N to a shared layer). It also
contradicts the project's whole premise — if we don't share an index
across languages, beans is just five LSPs in a trench coat.

**JVM projection as the only model, language models as transient ASTs.**
Parse into a rich language AST, lower it to the JVM projection, throw the
AST away. Rejected because within-language features (Kotlin-aware
completion, Scala-aware diagnostics) need persistent access to the rich
model. Lowering to JVM and discarding the source-level structure makes
those features impossible without re-parsing on every query.

**Lazy/on-demand projection.** Keep only language models; compute the
JVM projection on demand when cross-language queries arrive. Rejected
as a primary design (we want the JVM projection in the index for fast
cross-language queries) but worth keeping in mind as an optimization
for less-common projections.
