# ADR-0016: Run pipeline phases before registration; do not use event-driven processors

## Status

Accepted

## Context

A node that enters the graph is not raw parser output. It needs work
done on it first: type references resolved against imports, nullability
applied, generics elaborated, JVM projection computed, descriptors
generated. The question is **when** that work happens and **who**
triggers it.

Two shapes are common in this kind of system:

1. **Event-driven processors.** The raw node is registered. Processors
   subscribe to registry events ("a new Kotlin class appeared") and
   mutate the node — e.g., a "JVM projector" subscribes to all Kotlin
   class events and adds a JVM projection child. The processor itself
   may register the projection, which triggers other processors, and
   so on. RxJava, Salsa-with-derived-queries, and ECS systems lean
   this way.

2. **Synchronous enrich-then-register.** The raw node is taken through
   whatever phases are needed (resolve, project, enrich) **before** it
   is registered. By the time the registry sees it, it is fully formed.
   No processor subscribes to its appearance.

Shape 1 looks elegant on a whiteboard. In practice it produces
re-trigger loops that are hard to reason about and harder to debug.
Processor A subscribes to events from registry X, mutates nodes in a
way that triggers events on registry Y, which a processor B subscribes
to, which feeds back into X. The order of execution becomes
emergent. Stale-marking interacts with mid-pipeline state. Tests
become non-deterministic because phase ordering depends on event
arrival order.

We surveyed the prior art (RxJava-based pipelines, Salsa-style derived
queries, ECS systems) and the same pattern shows up: event-driven
mutation graphs work for small systems and then become the dominant
source of bugs as they grow. The bugs are the kind that are hard to
fix, because the fix is "rearchitect the whole pipeline."

The relevant cross-file dependency is real, though: a Java class that
references a Kotlin extension function cannot fully resolve until the
Kotlin file is parsed and registered. We need to handle that without
event-driven processors.

## Decision

Each language has a synchronous **enrich** path that runs to completion
before the node is registered. For example:

```rust
fn create_kotlin_class_node(parsed: ParsedKotlinClass, ctx: &mut Context)
    -> KotlinClassNode
{
    let resolved_supertypes = resolve_types(&parsed.supertypes, ctx);
    let nullability         = apply_nullability(&parsed, ctx);
    let jvm_projection      = project_to_jvm(&parsed, &resolved_supertypes);
    KotlinClassNode {
        // ...
        supertypes: resolved_supertypes,
        nullability,
        jvm_projection,
    }
}
```

`enrich` is a regular function call. It is synchronous, it does not
subscribe to anything, and it does not register the node mid-flight.
When the constructed node is finally inserted into the graph, all of
its derived state is already present.

There are **no processors** that subscribe to registry events and
mutate nodes. Adding a JVM projection is a step in the Kotlin enrich
path, not a separate processor that watches the Kotlin registry.

Cross-file dependencies — the Java-references-Kotlin-extension case —
are handled by the **dynamic link** mechanism (see [ARCHITECTURE.md](../../ARCHITECTURE.md)):
the Java node has an unresolved query that returns "not yet" if the
Kotlin file has not been parsed. When the Kotlin file is later parsed
and its node is enriched and registered, the registry's subscriber
notification marks the Java node stale, and the next pull re-resolves
the link. This is push-staleness, not push-mutation. The Java node is
not modified by a Kotlin-side processor; it is marked stale and
recomputes itself on the next pull.

## Consequences

**Positive.**

- Pipeline behavior is debuggable. Enrich is a stack of regular
  function calls. Setting a breakpoint or printing intermediate state
  is straightforward. There is no "wait for the next event" loop to
  reason through.
- No re-trigger loops by construction. A node cannot trigger a
  processor that mutates it, because there are no processors. A node
  can only trigger staleness in *other* nodes, and staleness is a flag
  flip — it does not feed back.
- Tests are deterministic. Building a node has a fixed result given
  fixed inputs. There is no event ordering to control.
- Each language's enrich path is colocated. Kotlin enrich logic lives
  in the Kotlin crate; Java enrich logic lives in the Java crate. We
  do not have a "shared processors" graph spanning crates.

**Negative.**

- Enrich must be synchronous. If a phase needs information from another
  file (e.g., resolved supertype from a different language), the choice
  is "enrich without that information and rely on stale notifications
  to update later" or "block." We always pick the former: enrich runs
  with the information available, marks the result as conditional on
  the unresolved queries, and stale-recomputes when those resolve.
- There is no central place to insert "do this for every node of every
  language." If a future feature needs that — e.g., a global
  enrichment step — it has to be added to every language's enrich path
  individually. We accept this; ADR-0017 explicitly chooses to inline
  rather than abstract.
- The first-time cost of a pull is higher than in a fully cached
  event-driven system, because enrich runs synchronously when the node
  is first constructed. Subsequent pulls are cheap (the node is fresh
  until invalidated). We expect parse-time enrich to be negligible
  next to parse time itself.

## Alternatives considered

**Event-driven processors that subscribe to registry events and mutate
nodes.** Rejected. Re-trigger loops are the failure mode of every
non-trivial system in this shape. The bugs are the kind that surface
late and require architectural rework. We are choosing a shape that
makes them impossible by construction rather than a shape that requires
discipline to avoid.

**Two-phase: register raw, then a single explicit "enrich pass" walks
all unenriched nodes.** Closer to the chosen shape, but it introduces
a phase boundary visible to consumers (don't query during the raw
phase!) and makes incremental updates awkward — when a single file
changes, do we re-run the global enrich pass? Rejected.

**Coroutine-style enrich that suspends on cross-file dependencies.**
Tempting because it removes the "enrich with what's available, recompute
later" awkwardness. Rejected because it requires an executor, makes
enrich non-deterministic with respect to file load order, and pushes
us toward async machinery we do not otherwise need.

**A single shared "JVM projection" processor running across all
languages.** Conflates two separable decisions: (a) projection logic
shared across languages and (b) projection running as a processor.
We share the *logic* (utility functions like `descriptor_from_signature`)
but not the *processing model*. ADR-0017 covers this in detail.
