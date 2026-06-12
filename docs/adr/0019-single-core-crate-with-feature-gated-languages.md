# ADR-0019: Collapse the workspace into a single beans-core crate with feature-gated language modules

## Status

Superseded by [ADR-0030](0030-vertical-crates-engine-jvm-model-language-verticals.md)

## Context

The original workspace layout split each language and each major
subsystem into its own crate: `beans-lang-java`, planned
`beans-lang-kotlin`, planned `beans-jmod`, and a `beans-core` that
held only the shared model. This is the layout described in the
current `ARCHITECTURE.md` and the current `Cargo.toml`. It looks
clean from a distance — one crate per concern, language code clearly
walled off — but in practice we have to reckon with how the pieces
actually want to talk to each other.

Two forces push back against the per-crate split:

1. **The graph engine, the JVM model, and the language modules are
   coupled by definition.** The `NodePayload` enum (the union of all
   node types in the semantic graph) has to know about every variant
   that any language can produce — Java symbols, Kotlin symbols, JVM
   projections, CST nodes, view nodes. Splitting that enum across
   crates either forces a generic boxed payload (paying allocation
   and dispatch cost on every node) or a "model" aggregator crate
   that re-exports types from every language crate (a thin wrapper
   that exists only to dodge the cycle). Neither carries its weight.

2. **Language modules are parsers and rule sets, not products.** A
   user does not install `beans-lang-java` independently; they
   install beans (the LSP, the CLI) and that brings in whichever
   languages were compiled in. The crate boundary is a build-system
   artifact, not a product boundary.

What we *do* want is the ability to compile beans without all the
languages — a CLI tool that only needs JVM bytecode analysis should
not pay the parse-time, link-time, and binary-size cost of pulling
in tree-sitter for five grammars.

## Decision

Beans is one library crate, `beans-core`, containing:

- The graph engine (nodes, registries, hard/dynamic links).
- The JVM model and the JMOD reader.
- All language modules (Java, Kotlin, Scala, Groovy, Clojure) as
  feature-gated submodules.
- The union `NodePayload` enum, with each language's variants gated
  behind the same feature flag as the language module.

Languages are Cargo features:

```toml
[features]
default = ["java", "kotlin", "scala", "groovy", "clojure"]
java = ["dep:tree-sitter-java"]
kotlin = ["dep:tree-sitter-kotlin"]
# ...
```

Consumers depend on `beans-core` and pick features:

```toml
# An LSP server with everything
beans-core = { path = "../beans-core" }

# A CLI that only does JVM bytecode analysis
beans-core = { path = "../beans-core", default-features = false }
```

`beans-lsp` and any future `beans-cli` are thin consumer crates that
depend on `beans-core`. They contain only the consumer-specific glue
(LSP wire protocol, CLI argument parsing) — no model, no parsing, no
indexing.

## Consequences

**Positive.**

- The `NodePayload` enum lives in one place. No re-export shims, no
  generic boxing, no aggregator crate that exists to dodge cycles.
- Adding a sixth JVM language is one feature flag and one submodule.
  No new crate to register, no `Cargo.toml` plumbing, no test crate
  scaffolding.
- A minimal `beans-cli` build (only JVM bytecode, no source parsers)
  is achievable by disabling default features. Binary size and
  compile time scale with what the consumer actually uses.
- Cross-language work (rules that touch both Java and Kotlin) lives
  in one crate, and the compiler enforces consistency directly.
- Test crates collapse: there is no need for a separate
  `beans-test-java` to depend on `beans-lang-java`. Tests live in
  `beans-core/tests/` and pick features like any other consumer.

**Negative.**

- `beans-core` becomes a large crate. Compile times for the crate
  itself are longer than for any of the previous per-language crates
  individually. Mitigated by feature gates (a developer working on
  Kotlin can disable the others locally) but not eliminated.
- Feature flags are a maintenance surface. Every public type that
  references a language-specific variant has to be either gated or
  written generically. We accept this; it is the price of the
  unified model.
- The line between "what's in `beans-core` vs. `beans-lsp`" is now
  the only crate boundary that matters. We need to police it
  actively (see ADR-0020).
- Mocking out a language for a test is slightly harder — features
  are workspace-global within a `cargo test` invocation. In
  practice this has not bitten us; tests pick fixture languages
  directly.

## Alternatives considered

**Separate crate per language (the current layout).** This is what
`Cargo.toml` shows today. Rejected for the reasons above: the
`NodePayload` enum forces an aggregator crate or generic boxing, and
the crate boundary does not carry its weight as a product boundary.
The per-crate split is good hygiene in many projects; it is not the
right shape for this one because the model is shared by definition.

**Separate `beans-graph` and `beans-jvm` crates with `beans-core`
as a thin re-exporter.** The graph engine and JVM model are big
enough subsystems to imagine giving them their own crates. Rejected
because they are not used independently. Every consumer of one
needs the other, and splitting them adds dependency edges with no
encapsulation benefit. If that ever changes — say, a downstream
project wants the JVM model without the graph engine — we revisit.

**A `beans-model` aggregator crate that re-exports types from each
language crate.** Solves the `NodePayload` enum problem by hosting
the enum in the aggregator. Rejected because it is a coordination
artifact: the aggregator has to be updated every time any language
adds a node variant, and the language crates can no longer be
compiled standalone (they would need a feature-gated `NodePayload`
of their own to satisfy `cargo check`). The per-crate split was
supposed to deliver isolation, and the aggregator immediately undoes
it. If we are going to share the enum, we may as well share the
crate.

**Keep per-language crates but make `beans-core` the home of
`NodePayload`, with each language crate providing only parser
helpers and rule registrations.** A "parser plugin" pattern.
Rejected because the parsers are not the bulk of a language module.
The bulk is the rules — type resolution, completion logic, hover
formatting — and those need direct access to `NodePayload`. Pushing
them into separate crates means either re-exporting the enum
(aggregator problem) or making rules generic over a payload trait
(the plugin-platform tax that ADR-0001 explicitly rejects).
