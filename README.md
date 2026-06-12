# Beans

Beans is a multi-language LSP for JVM languages, written in Rust. It targets Java, Kotlin, Groovy, Scala, and Clojure with a single shared index, so navigation, references, and refactors work across language boundaries instead of stopping at them.

## The killer feature: cross-language navigation

Real-world JVM projects mix languages constantly — Java with Kotlin (Android, Spring), Java with Groovy (Gradle, Spock), Scala with Java (data platforms). Every language boundary is a blind spot for separate LSPs:

- Renaming a Java interface method does not update its Kotlin implementation.
- Find-references on a Java class misses the Groovy test that calls it.
- Jumping from a Kotlin call site into the Java definition either fails or each LSP reimplements the other language's understanding from scratch.

Beans builds one symbol index that all five languages parse into. Go-to-definition, find-references, and (eventually) rename work across every JVM language in the project. No lightweight LSP outside IntelliJ does this today.

## Design principles

Two foundational decisions shape the architecture; the full set lives in [`docs/adr/`](docs/adr/).

- **Cohesive, not extensible** ([ADR-0001](docs/adr/0001-cohesive-not-extensible.md)). The five target languages are baked in. There is no plugin API, no dynamic language registry, no generic abstraction for hypothetical future languages. Adding a sixth JVM language is a code change in this repository, not a downstream extension point. The cost of an open plugin platform (IntelliJ-style) shows up in every line of code; we never get the benefit because the language set is closed in practice.
- **Library-first, LSP as a leaf consumer** ([ADR-0002](docs/adr/0002-library-first.md)). Beans is a Rust library that happens to ship an LSP server. `beans-lsp` is a leaf in the dependency graph — nothing imports it. Batch analyzers, CLIs, and IDE plugins that bypass LSP can sit on `beans-core` directly without going through JSON-RPC.

## Project status

Early-stage. The architecture is defined and the foundational layers exist; semantic analysis and the second language are not yet started.

- Implemented: shared symbol model, symbol table, resolution (imports, same-package, wildcard, static, compound), Java parser via tree-sitter, Java `Language` impl, LSP server with go-to-def / hover / references / document symbols, fixture-based test harness.
- Not yet implemented: Java semantic analysis (type inference, scope, flow), JDK class-file / `.jmod` reader, any language other than Java.

The project does not yet have a stable release or installable artifact. If you want to use it day-to-day, you build from source.

## Workspace layout

| Crate | Purpose |
|-------|---------|
| `beans-core` | Shared JVM symbol model, symbol table, `Language` trait, resolution. |
| `beans-lang-java` | Java source parser (tree-sitter-java) and `JavaLanguage` impl. |
| `beans-lsp` | LSP server. The only crate that knows about JSON-RPC. |
| `beans-test-harness` | Fixture-based test framework, language-agnostic. |
| `beans-lang-java-test` | Java spec tests built on the harness. |

## Getting started

Requirements: Rust (edition 2024).

```sh
cargo build --workspace
cargo test --workspace
cargo run -p beans-lang-java -- path/to/File.java   # parser CLI
cargo run -p beans-lsp                              # LSP server on stdio
```

The fixture test framework is the primary way behavior is encoded. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the tutorial and the test discipline.

For how to set up a development environment and submit changes, see [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Where to learn more

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — current technical reference: data model, graph engine, registries, lifecycle, crate layout.
- [`docs/adr/`](docs/adr/) — architecture decision records. Read these before proposing structural changes.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — contribution guidelines, fixture framework tutorial, test discipline.
- [`backlog/`](backlog/) — pending and completed work items.
