# Beans — Architecture Map

Beans is a Rust library that indexes the JVM language family (Java,
Kotlin, Groovy, Scala, Clojure) into one semantic graph and answers
editor queries — definitions, references, diagnostics, fixes — across
all of them. The library is the product; the LSP server is one consumer.

This file is a **map, not a manual**. The manual is the code: each
crate's `lib.rs` carries its architecture notes, and module docs explain
local invariants next to the code that enforces them. Rationale — why
things are shaped this way and what was rejected — lives in
[docs/adr/](docs/adr/README.md). If this file disagrees with the code,
the code wins; fix this file.

## Crates

| Crate | Job | Read first |
|---|---|---|
| `beans-core` | The symbolic engine: graph arena, `Registry<K>` + query types, `Location`, neutral analysis values (`Diagnostic`, `Fix`). No language or JVM knowledge. | `src/lib.rs`, `src/graph/`, `src/registry/mod.rs` |
| `beans-lang-jvm` | The shared JVM model every language projects into, plus `JvmRegistries` — the only registry surface verticals share. The `container` module reads class bytes out of `.jmod`/`.jar` archives (classfile decoding deferred to #012). | `src/lib.rs`, `src/container.rs` |
| `beans-lang-java` | The Java vertical: model, tree-sitter walker, resolution, diagnostic rules, fixes, `JavaRegistries`. | `src/lib.rs`, `src/parser.rs` |
| `beans` | The facade: the `NodePayload` union, the composed `Registries`, per-extension dispatch, the `Beans` instance. Languages are Cargo features here. | `src/lib.rs` |
| `beans-lsp` | The LSP rim: protocol envelopes over the facade, an actor bridging async tower-lsp to the single-threaded engine. | `src/actor.rs` |
| `beans-test-harness` | Fixture framework: `<cur>` markers, `.resolve()` / `.complete()` / `.diagnostics()` / `.quick_fix()`. | `src/fixture.rs` |
| `beans-lang-java-test` | Java spec tests, organized by JLS chapter, plus fix-behavior tests. | `tests/spec/` |
| `beans-test-jdks` | Test-only JDK provisioning: download + cache a pinned Temurin so container/bytecode tests don't trust `$JAVA_HOME`. | `src/lib.rs` |

```
beans-lang-java ─┐
beans-lang-kotlin┼──▶ beans-lang-jvm ──▶ beans-core
      (later)   ─┘
        ▲ composed by `beans` ◀── beans-lsp, tests, future CLIs
```

## Cross-crate invariants

The few rules no single module doc can own:

1. **Library-first.** Nothing depends on `beans-lsp`; it is a leaf.
   Anything a non-LSP consumer would want lives below the rim
   (ADR-0002, ADR-0020).
2. **Verticals never import each other.** Cross-language visibility
   exists only through the JVM projection and `JvmRegistries` —
   enforced by the crate DAG (ADR-0004, ADR-0030).
3. **The engine knows no languages.** `beans-core` contains no JVM or
   language types; the closed unions live in the `beans` facade
   (ADR-0030).
4. **Single-threaded engine.** `Rc<RefCell<_>>` inside; parallelism at
   the file-batch parse boundary via rayon; async only at the LSP edge
   (ADR-0005, ADR-0018).
5. **Stale-while-revalidate.** Never block the user on the engine
   catching up; serve last-known and reconcile (ADR-0028).

## Where to learn what

- Graph, hard links, RAII handles — `beans-core/src/graph/` module docs.
- Registries, queries, subscriptions, re-entrancy — `beans-core/src/registry/mod.rs`.
- JVM projection and promoted enrichments — `beans-lang-jvm/src/lib.rs`.
- Java IR (declarations, use sites, candidate FQNs) — `beans-lang-java/src/payload.rs`.
- Diagnostics and fixes — `beans-lang-java/src/{diagnostics,fixes}.rs`.
- The async/sync actor bridge — `beans-lsp/src/actor.rs`.
- Testing discipline — [CONTRIBUTING.md](CONTRIBUTING.md) and ADR-0022…0026.
