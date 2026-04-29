# ADR-0017: No central pipeline machinery; each node type owns its enrich

## Status

Accepted

## Context

ADR-0016 settles the question of *when* enrichment runs (synchronously,
before registration). This ADR addresses *how it is structured*. Given
that we have five languages and they all do roughly the same shape of
work — resolve types, apply enrichments, project to JVM, generate
descriptors — there is a clear pull toward shared pipeline machinery:

- A `Pipeline` trait with composable phases.
- A registry of phases per language.
- A trait `EnrichmentPhase` with `apply(&self, &mut Node)`, with shared
  phase implementations like `JvmProjectionPhase`, `NullabilityPhase`.
- An abstract `enrich(&self, node) = phases.iter().for_each(apply)` core
  with each language declaring its phase order.

This shape is easy to imagine because it is what the Java/Spring world
trains people to build. It also fails the test of ADR-0001 (cohesive,
not extensible). The five languages do *roughly* the same shape of
work, but the differences are precisely what make each language non-
trivial:

- Kotlin's nullability phase needs Kotlin-specific type-system context
  that no other language has.
- Scala's implicit resolution feeds back into type inference in a way
  that Java's type resolution does not.
- Clojure has no static types in the same sense; its "enrichment" is
  largely about var resolution and protocol dispatch.

A shared phase that abstracts over these is either lowest-common-
denominator (and useless because the language-specific work happens
inside an `Option<dyn LanguageEnricher>` escape hatch anyway) or full
of language-specific conditionals. Either way, the abstraction earns
its keep only after we have the same phase implemented across multiple
languages — and we do not yet.

We do, however, have shared *utility* logic that is genuinely cross-
language: computing JVM erasure from a `TypeRef`, generating a JVM
descriptor from a signature, name-mangling rules for inner classes.
That is shared *code*, not a shared *pipeline*.

## Decision

There is no central pipeline framework. Each node type owns its enrich
function:

```rust
impl KotlinClassNode {
    fn enrich(parsed: ParsedKotlinClass, ctx: &mut Context) -> Self {
        let supertypes = resolve_types(&parsed.supertypes, ctx);
        let nullability = apply_kotlin_nullability(&parsed, ctx);
        let projection = jvm_utils::project_class(&parsed, &supertypes);
        Self { supertypes, nullability, projection /* ... */ }
    }
}
```

Shared work is exposed as **utility functions**, not phases:

- `jvm_utils::erasure(type_ref) -> JvmType`
- `jvm_utils::descriptor_from_signature(sig) -> String`
- `jvm_utils::project_class(parsed, supertypes) -> JvmProjection`

Each language's enrich function calls the utilities it needs, in the
order that makes sense for that language. There is no `Pipeline`
trait, no phase registry, no `EnrichmentPhase` trait, no shared
"compose phases" infrastructure. The abstraction starts and ends with
ordinary functions.

If two languages turn out to need an *identical* phase later — same
input, same output, same context — that phase is extracted as another
utility function. We do not introduce pipeline composition just because
two languages happen to share one step.

## Consequences

**Positive.**

- Each language's enrich path is readable as a sequential function.
  There is no "look up the configured phases for this language" lookup
  step before you understand what runs.
- Adding a language-specific quirk is local. If Kotlin needs an extra
  step that only matters for Kotlin, it goes into `KotlinClassNode::enrich`
  and nowhere else.
- Shared utilities have proper signatures. `descriptor_from_signature`
  takes a `Signature` and returns a `String`. It does not take a `&mut
  dyn EnrichmentContext` and pretend to be polymorphic over things it
  is not.
- Refactoring a single language is genuinely local. We do not have to
  worry that changing Kotlin's enrich order will break Scala because
  they share a phase trait.
- The abstraction we do not build is one we cannot get wrong. There
  are no phase-ordering bugs, no half-applied phases, no "the
  pipeline ran in a weird order on Tuesday."

**Negative.**

- We cannot say "run enrich phase X across all languages." Cross-
  cutting changes (e.g., add a new universal enrichment step) require
  editing each language's enrich function. With five languages this
  is fine. With fifty it would be a problem; we are not in that regime
  (ADR-0001).
- Two languages may end up with *similar but not identical* logic for
  a step that should be shared. The fix is to extract a utility once
  the duplication is real; we accept some short-term duplication
  rather than premature abstraction.
- There is no global view of "what enrichments exist across the
  system." Discoverability comes from reading each language crate's
  enrich function, not from a registry. Documentation and code search
  cover the gap.

## Alternatives considered

**Shared "JVM projection" pipeline phase composed across languages.**
A single `JvmProjectionPhase` trait implemented per language, with a
shared composer that runs it. Rejected. The phase has different
prerequisites in different languages (Kotlin's projection needs
nullability info; Java's does not; Scala's needs implicit context).
The abstraction has to either accept a generic context that is
language-specific anyway, or split into per-language sub-traits, at
which point it is no longer "shared" in any meaningful sense.

**A `Pipeline` trait with `phases: Vec<Box<dyn Phase>>` per language.**
The structure looks composable on paper. Rejected because the cost
(a phase trait, dynamic dispatch, an ordering problem, error handling
across phases) is paid up front whether or not we ever benefit, and we
are not confident the benefit materializes. We can always add this
later if a real shared use case emerges. We cannot easily remove it.

**Macro-generated enrich functions.** A macro that expands into the
right sequence of utility calls per language. Rejected as gold-plating.
The function bodies are short enough that a macro adds opacity without
saving meaningful lines.

**Strategy pattern: each language registers an enrich strategy with a
central enricher.** Same problem as the trait pipeline, with extra
indirection. Rejected.
