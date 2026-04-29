# ADR-0008: Carry an ordered list of fallback queries on each dynamic link

## Status

Accepted

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

A dynamic link carries an **ordered list of registry queries** plus a
mode that determines how the queries combine.

- **`FirstMatch` mode (resolution).** The first query that returns a
  hit provides the value. Lower-priority queries are not consulted.
  This is what go-to-definition and type-checking use: a single
  authoritative target.

- **`MergeAll` mode (completion).** All queries fire and their
  results union, with a deterministic dedup rule (language-specific
  results win over generic JVM projections for the same symbol). This
  is what completion uses: every plausible candidate.

A typical Java-side reference to a method named `process` on a
`Service` value carries queries like:

```
[
  JavaRegistry("com.example.Service.process"),
  JvmRegistry("com.example.Service.process"),
]
```

If the first query hits (Java-defined method), Kotlin's projection is
ignored. If it misses (Kotlin-defined method), the JVM projection is
used. The use site is identical in both cases — only the active query
index differs.

The link tracks subscriptions tiered by query position: the active
query has a value-watch (fires on value change), higher-priority
queries have existence-watches (fires if a hit appears that would
override), lower-priority queries are ignored while a higher one is
active.

## Consequences

**Positive.**

- Cross-language resolution is uniform. Java→Kotlin, Scala→Groovy,
  any combination — the use site just lists the registries it's
  willing to consult, in priority order.
- The registry layer is dumb. It maps `key → NodeId` and notifies on
  changes. It doesn't know about precedence, fallback, or language
  pairs. All cross-language policy lives at the link, where the use
  site knows what it's looking for.
- Refactors that move a definition between languages just-work. The
  use site's query list doesn't change; only the active index moves.
- Adding a new language is local: add the language's registries and
  document which queries make sense at use sites in other languages.
  No central routing table.
- Completion gets the same machinery as resolution, just with a
  different combine mode. We don't have two parallel pipelines.

**Negative.**

- Each dynamic link is a small object (vector of queries, not a
  pointer). At the scale of a real project this is millions of
  links, each maybe a couple of allocations. We need to keep the
  query objects compact (small string, small enum, small key) and
  reuse them where possible.
- The use site decides the priority order. If a language's parser
  picks a wrong order, lookups produce surprising answers ("why did
  this resolve to the Java symbol when there's a Kotlin extension?").
  This is parser-author responsibility; not a system flaw, but a
  source of bugs that can be subtle.
- Subscription tiering is correct but non-trivial: existence watches
  on inactive higher-priority queries must be maintained and torn
  down as the active index moves. The implementation must be careful
  about leaks here.

## Alternatives considered

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
The link object centralizes this so rule authors get consistent
behavior for free.
