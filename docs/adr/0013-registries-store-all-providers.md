# ADR-0013: Registries store all providers; precedence is a resolution concern

## Status

Accepted

## Context

When a Java file imports `com.example.Service` and another classpath
entry also exports `com.example.Service`, somebody has to decide which
one wins. Same problem within a language: in Kotlin, a member function
shadows an extension function with the same shape; in Scala, an explicit
import beats a wildcard import; in Clojure, the most recently `require`-d
namespace wins for unqualified references. Every JVM language has its
own rules.

A registry that returns "the symbol for FQN X" has to either:

1. Pick a winner internally (the registry knows the precedence rules), or
2. Return all providers and let the caller decide.

Option 1 is tempting. It centralizes the rule. The caller's life is
simple: `registry.lookup(fqn)` returns the right answer. But it pushes
language-specific logic into the registry, which is supposed to be the
shared substrate. Java's classpath shadowing rules differ from Kotlin's
import precedence rules, which differ from Clojure's `require` rules.
A registry that knows all of those is no longer language-agnostic; it
is a pile of conditionals.

Worse, the registry would have to know **which language is asking**.
"What does `Service` mean?" has different answers depending on whether
the lookup originates from a Java import or a Kotlin import. The
registry would need a request context, the request context would need
language tags, and the simple key-to-value mapping turns into a
multi-dimensional decision table.

## Decision

Registries store **all providers** for each key, with no notion of a
winner:

```rust
struct Registry<K> {
    providers: HashMap<K, Vec<NodeId>>,
    subscribers: HashMap<K, Vec<Subscription>>,
}
```

A registry's `query(key)` returns the entire provider list. The
registry has no `query_one` operation that picks a winner. Order within
the `Vec` is insertion order; it carries no semantic weight.

Picking a winner is a **resolution-layer** concern. Java's classpath
shadowing, Kotlin's import precedence, Scala's `package object` lookup,
Clojure's `require` precedence — all of these are implemented as
language-specific resolution code that calls `query` and applies its
own rules to the resulting list.

The `query_one` vs `query_many` distinction lives at the resolution
layer, not the registry layer. `Ambiguous` is a resolution outcome
(two equally valid candidates after applying language rules), not a
registry outcome (which never decides).

## Consequences

**Positive.**

- The registry is genuinely language-agnostic. The same `JvmRegistry`
  serves all five languages without knowing anything about their
  respective import or shadowing rules.
- Language-specific precedence is expressible as ordinary Rust code
  next to the rest of the language's resolution logic. It is testable,
  debuggable, and changeable without touching shared code.
- Multiple resolutions of the same key with different rules (e.g., a
  refactoring tool that wants "all candidates" vs. an LSP go-to-def that
  wants "the one the compiler would pick") can both call the same
  registry and apply different post-processing.
- Diagnostics that need to enumerate ambiguities ("did you mean X or Y?")
  get the raw provider list directly.

**Negative.**

- The simplest case — "there's exactly one `com.example.Foo`" — pays a
  small allocation cost (a `Vec` containing one `NodeId`) and a small
  unwrap step in the caller. We measured this is not the bottleneck;
  the bottleneck is resolution logic, which dominates.
- Resolution code is more verbose. Every call site has to handle the
  vec-of-providers shape rather than `Option<NodeId>`. We mitigate this
  with helper functions in the resolution layer (e.g.,
  `pick_one_by_classpath_order`) that encapsulate common patterns.
- There is no global "wrong" detection at the registry level. If the
  resolution code has a bug and picks the wrong provider, the registry
  cannot catch it. We accept this; the registry was never the right
  place for that check.

## Alternatives considered

**Precedence built into the registry (Vec sorted by precedence).** Each
registry entry carries a precedence number, and the registry returns
them sorted. Rejected because precedence is not a property of a provider
— it is a property of a query, in a context. The same Java class has
different precedence depending on whether the query originates from
inside vs. outside its package, from a wildcard import vs. an explicit
import, etc. There is no single ordering.

**Registries return one provider plus shadowed list.** Caller gets the
"winner" plus the shadowed candidates. Rejected because the registry
still has to know how to pick. Same problem as having precedence
internal, with extra ceremony.

**Per-language registries with their own precedence built in.** Each
language has its own `Registry` type with its own rules. Rejected
because cross-language registries (especially JVM) would still need a
way to handle multi-language ambiguity, and we would end up with both
patterns. One uniform substrate is simpler.

**Registry returns `Result<NodeId, Ambiguous>` and language code uses
`Ambiguous` to disambiguate.** Closer, but it commits to one outcome
shape (single answer or ambiguous error) and forces ambiguity to be an
error rather than a normal case. Completion needs to merge candidates
from multiple registries — that is not an error, it is the feature.
Rejected because it bakes a single resolution mode into the registry.
