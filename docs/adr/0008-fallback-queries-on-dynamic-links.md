# ADR-0008: Carry an ordered list of fallback queries on each dynamic link

## Status

Accepted (rev 2 — implementation shape clarified)

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

We also need this to apply to completion, not just resolution. When a
user types `service.<cur>`, we want all candidates — Java methods on
`Service`, Kotlin extensions targeting it, JVM-projected members from
other-language definitions — surfaced in one merged list.

## Decision

A use-site stores **the question, not the answer**: an ordered list
of registry queries that resolve to provider `NodeId`s on demand.
Three layers cover the actual implementation, sized to their callers:

1. **`Queryable<M>` trait + `first_match` / `all_matches` helpers**
   (`beans-core/src/query.rs`). For *stateless* one-shot queries.
   `Queryable<M>` is the trait registries implement once per query
   shape they answer (native key, plus cross-registry models like
   `ByFqn`). `first_match(model, &[&dyn Queryable<M>])` walks a
   priority list of registries and returns the first hit;
   `all_matches` returns the union. No state, no cache, no
   subscription — every call re-resolves.

2. **Closed `RegistryQuery` enum** (`beans-core/src/multi_query.rs`).
   For *heterogeneous* lists: each variant carries a typed key for one
   registry. `RegistryQuery::providers(&beans)` and
   `RegistryQuery::subscribe(&beans, cb)` dispatch through a closed
   match — adding a new registry adds one variant and the compiler
   flags every site that needs the new arm.

3. **`MultiQuery`** (`beans-core/src/multi_query.rs`). For *stored,
   subscription-backed* use-site queries. Holds a `Vec<RegistryQuery>`
   plus a cache; subscribes to each underlying registry on
   construction so any provider-set change flips the cache to `Stale`
   and fires the MultiQuery's own subscribers (consumers reach
   `MultiQuery::subscribe(cb)` with the same RAII shape as
   `Registry::subscribe`).

A typical Java-side reference to a method named `process` on a
`Service` value, expressed as a `MultiQuery`:

```rust
let mq = MultiQuery::new(&beans, vec![
    RegistryQuery::JavaSymbol(JavaSymbolKey::new("com.example.Service.process")),
    RegistryQuery::JvmMethod(JvmMethodKey::new(owner, "process", erased_params)),
]);
let _watch = mq.subscribe(/* on-change callback */);
// later: mq.query(&beans) -> QueryResult
```

If the first query hits (Java-defined method), the JVM projection is
ignored. If it misses (Kotlin-defined method), the JVM projection is
used. The use site is identical in both cases.

`MultiQuery::query` returns a [`QueryResult`] tri-state — `None`,
`One(NodeId)`, or `Many(Vec<NodeId>)` — making cardinality explicit
at every call site. `providers_all` covers the merge-all (completion)
combine mode without caching, since completion answers change too
often for a stale Vec to be useful.

Per-query subscription tiering (value-watch on the active query,
existence-watch on higher-priority queries that currently miss,
unobserved lower-priority queries) is a future optimisation. The
current implementation subscribes to *every* underlying registry and
invalidates on any change. Coarse but correct; tiering lands when
profiling shows it's load-bearing.

## Consequences

**Positive.**

- Cross-language resolution is uniform. Java→Kotlin, Scala→Groovy,
  any combination — the use site just lists the registries it's
  willing to consult, in priority order.
- The registry layer is dumb. It maps `key → NodeId` and notifies on
  changes. It doesn't know about precedence, fallback, or language
  pairs. All cross-language policy lives at the use site, where the
  language is known.
- Refactors that move a definition between languages just-work. The
  use site's query list doesn't change; subscriptions trigger the
  invalidation; the next read picks the new active answer.
- Adding a new language is local: add the language's registries (one
  field on `Registries`), add a `RegistryQuery` enum variant, and
  document which queries make sense at use sites in other languages.
  Compiler flags every match site that needs the new arm. No central
  routing table.
- Completion gets the same machinery as resolution. The trait + helper
  combo handles stateless one-shots; `MultiQuery::providers_all` covers
  cached cross-registry merges.
- The tri-state `QueryResult` makes cardinality explicit (caller never
  has to inspect a Vec to ask "did anything match? exactly one
  thing?"); the zero/one cases never allocate.

**Negative.**

- Three layers feel like a lot for "do a registry lookup." The
  layering is sized to caller need: a stateless lookup uses the
  trait + helpers; a stored cached lookup uses `MultiQuery`. Pick the
  smaller layer that fits.
- The closed `RegistryQuery` enum has one variant per registry. When
  registries proliferate (Kotlin, Scala, Groovy, Clojure each add at
  least one), the enum + every match block grows in lockstep. This is
  cohesive-not-extensible (ADR-0001) playing out as expected.
- Each `MultiQuery` holds N subscription handles (one per consulted
  registry). At the scale of "millions of use-sites" this is real
  memory; revisit when profiling shows it. Coarse per-key
  subscriptions today; tiered subscriptions later.
- The use site decides the priority order. If a language's parser
  picks a wrong order, lookups produce surprising answers. This is
  parser-author responsibility; not a system flaw, but a subtle bug
  source.

## Alternatives considered

**`DynamicLink` as a single conflated abstraction.** Earlier revs of
this ADR proposed `DynamicLink<Q>` — a generic struct holding queries,
mode (FirstMatch/MergeAll), cached active index, and manual
`invalidate()`. Implemented in step 3 of the migration; trimmed in a
later commit (see the "Trim DynamicLink to RegistryQuery" commit) when
review surfaced that it bundled three orthogonal concerns (cardinality,
heterogeneity, caching) into one type for consumers that didn't yet
exist. Replaced by the three-layer split above, sized to actual use.

**One registry per source language; no fallback.** Java code only
queries `JavaRegistry`. Kotlin code only queries `KotlinRegistry`.
Cross-language calls return "unresolved." Simple and obviously wrong:
in real JVM projects, cross-language calls are constant. Java users
calling Kotlin libraries is the entire reason JVM interop exists.
Rejected.

**Registry-level precedence, single query per link.** The link asks
one registry; the registry internally knows about other registries
and handles fallback. Rejected because it puts language-pair logic
inside `JvmRegistry` (or wherever we centralize it), creating a
coupling between every language and a single routing arbiter. Adding
a new language means modifying the central arbiter — exactly the
"central registry of all language pairs" antipattern. Distributing
this knowledge to use sites (which already know the languages
involved) is healthier.

**Try registries in fixed global order.** All links try `JavaRegistry`
first, then `KotlinRegistry`, then `ScalaRegistry`, then `JvmRegistry`.
Simple, but forces every language pair to live with the same
priority. Kotlin extension functions, for example, want to win over
JVM projections — but with global order, they might not. Rejected
because it reduces a per-link policy decision to a single global
ordering that can't satisfy all callers.

**Embed combine logic in the rule code.** Each rule decides whether
to call the Java registry first or the Kotlin one, ad hoc. Rejected
because we'd duplicate fallback logic in every rule, and small
inconsistencies between rules would produce silent semantic drift
("why does completion show this but go-to-definition can't find it?").
Centralising this in `MultiQuery` (or its stateless siblings) means
rule authors get consistent behavior for free.

[`QueryResult`]: ../../beans-core/src/query.rs
