# ADR-0033: Compose every language vertical unconditionally; no language Cargo features in the facade

## Status

Accepted. Amends [ADR-0030](0030-vertical-crates-engine-jvm-model-language-verticals.md),
which originally made languages Cargo features of the `beans` facade.

## Context

ADR-0030 split the workspace into engine / shared JVM model / per-language
verticals, with the `beans` facade composing them. To leave room for partial
builds it made each language a Cargo feature of the facade —
`default = ["java", "kotlin", "scala", "groovy", "clojure"]` — and gated the
per-language arms of the `NodePayload` union, the composed `Registries`, and
per-extension dispatch on those flags. Placeholder features stood in for
verticals whose crates do not exist yet, backed by empty
`languages::{kotlin,scala,groovy,clojure}` modules.

That was reasonable while we were still exploring whether partial builds
mattered. In practice it put a feature matrix on the one crate whose whole
purpose is cohesion. Beans' thesis (ADR-0001) is cohesive cross-language JVM
support: the LSP processes every supported language together, and *which*
languages a given workspace happens to contain is a runtime fact the
per-extension dispatch already decides. The Cargo features modelled a
"which languages are present" axis the product does not actually have. The
placeholder features were strictly worse — they gated empty modules, so a
`kotlin` feature enabled nothing.

Cargo features earn their place when they protect an optional dependency edge
or a meaningful alternative build. The facade's language features did neither
in the steady state: every real consumer (the LSP, the spec tests, the
harness) turned them all on, and the only "alternative build" — a
bytecode-only facade — is better served by depending on the lower-level
crates directly.

## Decision

The `beans` facade composes every supported language vertical
unconditionally. There are no language Cargo features.

- `beans/Cargo.toml` declares no `[features]` table; `beans-lang-java` (and
  the parse-time `rayon` / `walkdir`) are plain, non-optional dependencies.
- The `NodePayload` union, the composed `Registries`, the `view` projections,
  the `Workspace` engine, and `compute_diagnostics` carry their Java arms
  unconditionally. A new language adds an arm when its crate lands.
- The placeholder features and the empty
  `languages::{kotlin,scala,groovy,clojure}` modules are removed until those
  `beans-lang-*` crates exist; each language's kinds return with its crate.
- Consumers (`beans-lsp`, `beans-test-harness`, `beans-spec-tests`) depend on
  plain `beans` and no longer mirror facade language features.

The minimal-build / library escape hatch is unchanged and lives one layer
down (a non-goal to remove): a consumer that does not want the whole composed
product depends on `beans-core`, `beans-lang-jvm`, or an individual
`beans-lang-*` crate directly, rather than partially compiling the facade.

## Consequences

**Positive.**

- One composed API shape. No conditional public fields or variants — every
  reader and consumer sees the same `beans` regardless of build flags.
- The feature matrix the facade and its consumers carried is gone, and with
  it the mixed `--no-default-features` states ADR-0030 listed as a negative
  and the feature mirroring in the harness and spec-test crates.
- Placeholder noise is gone: nothing advertises a `kotlin` capability that
  does no work.

**Negative.**

- There is no partially compiled facade. A consumer that wants only the
  JVM/bytecode surface depends on the lower crates directly — a deliberate
  boundary, not the facade with features turned off.
- Adding a language is strictly an additive code change (a payload arm, a
  registry field, a dispatch arm) rather than feature wiring. This is the
  intended shape, but it means the facade always compiles every vertical
  that exists.

## Alternatives considered

**Keep per-language features (ADR-0030 as written).** Retains the option of
partial facade builds. Rejected: the option went unused, the steady state
turned everything on, and the placeholder features gated nothing — net
architectural noise on the product's central crate.

**Make the placeholder modules unconditional instead of deleting them.**
Keeps `beans::languages::kotlin` and friends present without a feature.
Rejected: with no backing vertical they are empty public modules — the same
noise without the gate. The kinds come back when their `beans-lang-*` crate
does.

**Runtime plugin loading or workspace-configured language sets.** Out of
scope and an explicit non-goal: language presence is a runtime fact decided
by per-extension dispatch, not configuration or dynamic loading.
