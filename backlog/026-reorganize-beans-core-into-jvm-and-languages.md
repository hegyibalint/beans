---
status: pending
area: core
priority: high
---

# Reorganize beans-core into jvm/ and languages/ modules

## Description

The prototype's JVM/Java-specific types currently live at `beans-core/src/`
top level: `Symbol`, `SymbolKind`, `Modifier`, `Relation`, `Signature`,
`TypeRef`, `AnnotationInstance`. Several of those (notably `SymbolKind`)
are also where Kotlin/Scala/Clojure variants got bolted on as the universal
enum approach showed its limits.

Move them into the per-layer module structure committed to in ADR-0019
and ADR-0004:

- `beans-core/src/graph/` — generic engine, already correct, no change.
- `beans-core/src/primitives/` — `Location` and any other genuinely
  cross-cutting types (the set is small).
- `beans-core/src/jvm/` — JVM interop layer: the "Symbol-like" struct,
  JVM `Modifier`, JVM `Relation`, JVM `Signature`, `TypeRef`,
  `AnnotationInstance`, `JvmSymbolKind`. Anything that maps onto the
  bytecode model.
- `beans-core/src/languages/<lang>/` — one module per supported language,
  each gated by a Cargo feature. Each owns its own per-language kind enum
  and any signature/modifier shapes the JVM projection can't capture
  (Kotlin nullability, Scala given/HKT, Clojure protocols, etc.).

Cross-language consumers see only the JVM-layer types. Within-language
consumers reach into the relevant language module. This matches the split
ADR-0004 commits to and removes the universal-enum bloat from the top of
the crate.

## Context

Backed by ADR-0019 (collapse the workspace into one beans-core with
feature-gated language modules) and ADR-0004 (per-language models with
shared JVM projection). ADR-0021 (preserve tree-sitter walker, rewrite
the layers above) puts this in the rewrite pile.

The graph engine landed first because everything else builds on it; this
is the next major structural chunk. Recommend doing it before any
implementation milestone that introduces new node payloads, so the new
work lands in the right module from day one and we don't have to move it
twice.

## Acceptance criteria

- `beans-core/src/jvm/`, `beans-core/src/primitives/`, and
  `beans-core/src/languages/{java,kotlin,scala,groovy,clojure}/` exist
  with the right modules.
- The current `Symbol` / `SymbolKind` / `Modifier` / `Relation` /
  `Signature` / `TypeRef` / `AnnotationInstance` modules are gone from
  the top level of `beans-core/src/`. The replacements live under
  `jvm/` (and per-language modules where appropriate).
- `SymbolKind`'s Kotlin/Scala/Clojure variants are split into per-language
  kind enums in their respective language modules; the JVM-layer kind
  enum carries only kinds that the JVM projection cares about.
- Each language module is gated by a feature flag in `Cargo.toml`.
- All 428+ tests still pass.
- `cargo doc -p beans-core --no-deps` is clean.
- Existing imports across the workspace are updated to the new paths;
  no API regression beyond path changes.

## Notes

This is a big mechanical refactor with no behaviour changes. Best done
as one focused milestone with an agent driving the moves and a CodeRabbit
review pass at the end.

It also unblocks reasonable per-language documentation: once the Kotlin
kinds live in `languages/kotlin/`, citing the Kotlin spec from those docs
is natural. Today they sit under `SymbolKind` next to Java variants and
the citations get awkward.
