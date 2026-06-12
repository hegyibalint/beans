# ADR-0030: Split the workspace into engine, shared JVM model, and per-language vertical crates

## Status

Accepted. Supersedes [ADR-0019](0019-single-core-crate-with-feature-gated-languages.md).

## Context

ADR-0019 collapsed the workspace into a single `beans-core` crate with
feature-gated language modules, primarily because the `NodePayload`
union must see every language's model types, and a per-language crate
split seemed to force either boxed payloads or an aggregator crate.

Two things changed since. First, the auto-import design sessions
sharpened the architecture vocabulary into **engine / language
verticals / consuming rims**: the engine (graph + registries) answers
binding questions; each language's analyses form a vertical that judges
those answers; verticals never call each other — they meet at the
shared JVM projection (ADR-0004) and its registries. That left the
single crate hosting three architecturally distinct regions with only
review discipline keeping them apart. In particular, nothing stopped
one language module from importing another's internals once both
features were enabled.

Second, the design work found the seams that make a split viable
without ADR-0019's feared escapes: walkers can be generic over the
payload (`P: From<JavaNodePayload> + From<JvmNodePayload>`), rules can
be generic over a small projection trait (`P: AsJava`), and the
registry bag decomposes per-vertical with no generics at all. The
union types still need a single home — but that home can be a thin
crate *above* the verticals rather than a fat crate containing them.

## Decision

The workspace is split along the architecture's own lines:

- **`beans-core`** — the symbolic engine: graph arena, the
  `Registry<K>` primitive and query types, `Location`, and the neutral
  analysis values (`Diagnostic`, `Fix`). No language and no JVM
  knowledge. There is no bag-of-registries here.
- **`beans-lang-jvm`** — the shared JVM model (payloads, enrichments,
  `TypeRef`/`Modifier`/`SymbolKind`, descriptor vocabulary) and
  `JvmRegistries`. This is the only registry surface shared across
  verticals; every vertical registers its projections here and sees
  other languages exclusively through it. The future jmod/bytecode
  reader (backlog #012) lands here.
- **`beans-lang-<language>`** — one crate per vertical, owning its
  model, walker, resolution, rules, and fixes, plus its own registry
  bag (`JavaRegistries`, ...). Depends on `beans-core` and
  `beans-lang-jvm` only. Each vertical exposes a projection trait
  (`AsJava`, ...) that generic vertical code is written against.
- **`beans`** — the facade. Owns the two closed unions
  (`NodePayload`, the composed `Registries`), the `From`/projection
  impls that connect them to the verticals, per-extension dispatch
  (`compute_diagnostics`), and the `Beans` instance. Languages are
  Cargo features of this crate.
- **`beans-lsp`**, test crates — rims; depend on the facade.

Dependency direction, compiler-enforced:

```
beans-lang-java ─┐
beans-lang-kotlin┼──▶ beans-lang-jvm ──▶ beans-core
       ...      ─┘
        ▲ composed by `beans` ◀── beans-lsp, beans-test-*
```

## Consequences

**Positive.**

- "Verticals never import each other" is enforced by the crate DAG,
  not by review. Any cross-language visibility must be expressed as a
  JVM projection — ADR-0004's discipline with teeth.
- The engine's "symbolic core" identity is structural: `beans-core`
  contains no language and no JVM types at all.
- Model and behavior of one language live in one crate; a language
  feature slice (e.g. a payload field plus the rule reading it) is a
  single-crate change.
- Heavy parser dependencies (tree-sitter grammars) compile only in
  their vertical.
- The registry decomposition mirrors resolution policy: a Java rule's
  context is `&JavaRegistries` + `&JvmRegistries` — the ADR-0008
  primary/fallback pair as crate ownership.

**Negative.**

- Generic seams: walkers and rules carry `P: From<...>`/`P: AsJava`
  bounds instead of matching a concrete union. Contained to vertical
  signatures; rims see only the concrete `NodePayload`.
- The facade is a new coordination point: adding a language touches
  the new vertical crate plus the facade's unions and features.
- Cargo feature unification can produce mixed states in whole-
  workspace `--no-default-features` builds (a member forcing
  `beans/java` on while the harness's own `java` flag is off). The
  supported min-build is `beans` with `default-features = false`.

## Alternatives considered

**Stay single-crate (ADR-0019).** Works, but keeps vertical isolation
as a convention and the engine's purity as vocabulary. Rejected once
the vertical architecture became the explicit design — the structure
should encode it.

**Minimal split: models stay in core, only behavior moves.** Avoids
all generics, but cuts every vertical in half at its busiest seam —
most language work touches model and behavior together — and leaves
core containing every language's model. Rejected.

**Boxed payload trait (`Box<dyn AnyPayload>`).** Avoids the union
entirely; pays allocation + dynamic dispatch per node and loses
exhaustive matching. Rejected, as in ADR-0019.
