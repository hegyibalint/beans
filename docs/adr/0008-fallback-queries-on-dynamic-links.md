# ADR-0008: Cross-language fallback through `FallbackSubscription`

## Status

Accepted (rev 3 — implementation shape simplified to a concrete type)

## Context

Cross-language references in a JVM project are common and irregular.
Java code calls Kotlin extension methods. Kotlin code calls Groovy
classes. Scala code uses Java libraries. From the use site, we can
only partially predict where the definition lives — we know it's a
method named `process` on something called `Service`, but the actual
definition could be:

- A Java method (resolves directly in the Java registry).
- A Kotlin method (resolves in the JVM registry via the Kotlin node's
  JVM projection).
- A Kotlin extension function on a `Service` receiver (resolves in
  the Kotlin extension registry).
- Missing entirely (unresolved reference; a diagnostic).

The use-site language doesn't always know which registry to consult.
Worse, the answer can change as the user adds and deletes files. A
reference that resolved through `KotlinExtensionRegistry` yesterday
might resolve through `JavaRegistry` today after a refactor. Our
resolution machinery has to handle this gracefully.

## Decision

A use-site stores **the question, not the answer**: a typed
[`FallbackSubscription<P, F>`] that bundles a primary subscription
(language-native key) with a fallback subscription (typically a JVM
projection key). Two-key fixed shape, statically dispatched, owns its
RAII subscriptions, watches both sides for invalidation.

The full query layer in `beans-core/src/registry/query.rs`:

1. **[`QueryResult`]** — tri-state owned value (`None`, `One(NodeId)`,
   `Many(Vec<NodeId>)`). Per ADR-0007 the `NodeId`s are generational
   handles, safe to hold across mutations and to dereference later
   through the graph's generation-validated `get`.

2. **[`Query<K>`] / [`Subscription<K>`]** — the typestate split for a
   single-key lookup. `Query<K>` is stateless: just resolve. Calling
   `Query::subscribe(cb)` consumes the query and returns a
   `Subscription<K>` whose `Drop` automatically removes the registry
   entry. The type *is* the lifecycle: a `Subscription` is, by
   construction, subscribed; a `Query` is, by construction, not. No
   `Option<SubscriptionId>` state machine.

3. **[`FallbackSubscription<P, F>`]** — two typed `Subscription`s,
   primary-then-fallback resolve, cached invalidation. Subscribes to
   both registries at construction; either's underlying provider-set
   change invalidates the cache and fires consumer subscribers
   registered via `subscribe(cb)`. Returns a [`Watch`] handle whose
   `Drop` stops further notifications.

A typical Java-side reference to a method named `process` on a
`Service` value:

```rust
let fb: FallbackSubscription<JavaSymbolKey, JvmMethodKey> = FallbackSubscription::new(
    &beans.registries.java_symbols,
    JavaSymbolKey::new("com.example.Service.process"),
    &beans.registries.jvm_methods,
    JvmMethodKey::new(owner, "process", erased_params),
);
let _watch = fb.subscribe(Rc::new(|| { /* on change */ }));
// later: fb.resolve() -> QueryResult
```

If the primary query hits (Java-defined method), the JVM projection is
ignored. If it misses (Kotlin-defined method), the fallback projection
is used. The use site is identical in both cases.

Per-query subscription tiering (value-watch on the active query,
existence-watch on the inactive higher-priority side) is a future
optimisation. The current implementation subscribes to *both* sides
and invalidates on any change. Coarse but correct; tiering lands when
profiling shows it's load-bearing.

## What this rejects

**A generic `MultiQuery<N>` over an arbitrary number of registries.**
Earlier revs of this ADR proposed exactly that — `DynamicLink<Q>` with
priority lists, then a closed `RegistryQuery` enum, then `Box<dyn Query>`
storage. Each iteration confronted the same friction: the abstraction
served a generality the project doesn't actually have. Across all five
JVM languages, the recurring pattern is exactly two queries —
language-native + JVM fallback. Naming that one pattern as a concrete
type (`FallbackSubscription<P, F>`) is more honest, statically
dispatched, zero `Box<dyn _>`, and reads better at every call site.

**A separate `SubscriptionHandle<K>` RAII guard.** The earlier shape
returned a guard from `Registry::subscribe`. The typestate split makes
the guard unnecessary: `Subscription<K>` *is* the RAII anchor. Drop
fires the registry's `remove_subscription` directly via a strong `Rc`
clone (no `Weak::upgrade` dance — the `Rc` keeps the registry alive
while the subscription exists, which is the desired ownership story).

**A public `Registry<K>::subscribe`.** Subscriptions go through
`Registry::query(key).subscribe(cb)`. The raw `subscribe_internal` is
`pub(crate)`. The query path is the only public way to start watching;
the registry doesn't need to expose the lower-level mechanism.

## Consequences

**Positive.**

- Cross-language resolution is named directly. `FallbackSubscription<P,
  F>`'s type signature documents the intent at every call site.
- The registry layer is dumb. It maps `key → NodeId` and notifies on
  changes. It doesn't know about precedence, fallback, or language
  pairs. All cross-language policy lives at the use site, where the
  language is known.
- Refactors that move a definition between languages just-work. The
  use site's primary/fallback keys don't change; subscriptions
  trigger the invalidation; the next read picks the new active answer.
- Adding a new language is local: add the language's registry field on
  `Registries`. `FallbackSubscription` works with any
  `(Registry<NativeK>, Registry<FallbackK>)` pair via its generic
  parameters. No central enum or match arms to update.
- The typestate split (`Query<K>` vs `Subscription<K>`) makes the
  watch/no-watch lifecycle explicit at the type level. The compiler
  enforces correct usage; no `Option<SubscriptionId>` state machine.
- The tri-state `QueryResult` makes cardinality explicit. The zero/one
  cases never allocate.

**Negative.**

- The `FallbackSubscription<P, F>` shape is fixed at two keys. A future
  composition needing a different shape (e.g., completion's "merge
  across N registries") gets its own concrete type — designed when
  we know what its consumers actually need, not speculatively.
- Each `Subscription` carries an `Rc` clone of its registry. At the
  scale of "millions of use-sites" this is real memory; revisit when
  profiling shows it. Coarse per-key subscriptions today; tiered
  subscriptions later.
- The use site decides which registry is primary vs fallback. If a
  language's parser picks a wrong order, lookups produce surprising
  answers. Parser-author responsibility; not a system flaw, but a
  subtle bug source.

## Alternatives considered

**Generic `MultiQuery<N>` with `Box<dyn Query>` storage.** Implemented
through several iterations on this branch and trimmed each time after
review surfaced the over-generality. Replaced by the concrete
`FallbackSubscription<P, F>` per the analysis above.

**Closed `RegistryQuery` enum (one variant per registry).** Hand-rolled
monomorphization — N variants × M methods of boilerplate. Worked, but
the variants were just `(Registry<K>, K)` with different K. A generic
struct (`Subscription<K>`) expresses this directly without an enum.

**Macro-generated registry manifest.** Considered as a way to
synchronize the `Registries` field list with a closed `RegistryQuery`
enum's variants. Rejected: macros are last-resort, and the
`FallbackSubscription` shape eliminates the parallel-arrays problem
entirely.

**One registry per source language; no fallback.** Cross-language
calls return "unresolved." Simple and obviously wrong: in real JVM
projects, cross-language calls are constant. Rejected.

**Registry-level precedence, single query per link.** The link asks
one registry; the registry internally knows about other registries
and handles fallback. Rejected because it puts language-pair logic
inside `JvmRegistry` (or wherever we centralize it), creating a
coupling between every language and a single routing arbiter. Adding
a new language means modifying the central arbiter — exactly the
"central registry of all language pairs" antipattern. Distributing
this knowledge to use sites (which already know the languages
involved) is healthier.

[`QueryResult`]: ../../beans-core/src/registry/query.rs
[`Query<K>`]: ../../beans-core/src/registry/query.rs
[`Subscription<K>`]: ../../beans-core/src/registry/query.rs
[`FallbackSubscription<P, F>`]: ../../beans-core/src/registry/query.rs
[`Watch`]: ../../beans-core/src/registry/query.rs
