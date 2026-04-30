# Beans

This file is the canonical coding-agent guidance for this repository. Keep tool-specific instruction files as symlinks or thin pointers to `AGENTS.md` so instructions do not drift.

Beans is a multi-language LSP for JVM languages (Java, Kotlin, Groovy, Scala, Clojure). See `README.md` for vision and project status, `ARCHITECTURE.md` for technical design, and `docs/adr/` for the design decisions behind it.

## Key Documents

- `README.md` - Project intro, vision, build/test commands.
- `ARCHITECTURE.md` - Current technical reference: data model, graph, registries, lifecycle, crate layout.
- `CONTRIBUTING.md` - Development setup, fixture framework tutorial, spec test discipline.
- `docs/adr/` - Architecture decision records. Read these before proposing structural changes.
- `backlog/` - Pending and completed work items, one file per item.

## Project Structure

```text
beans-core/           # Shared JVM symbol model, symbol table, Language trait, resolution
beans-lang-java/      # Java source parser (tree-sitter-java) + JavaLanguage impl
beans-lsp/            # LSP server (go-to-def, hover, references, document symbols)
beans-test-harness/   # Fixture test framework (language-agnostic, no language deps)
beans-test-java/      # Java spec tests (uses harness + JavaLanguage via prelude)
```

## Development

- Language: Rust edition 2024.
- Run tests: `cargo test --workspace`.
- Run Java parser CLI: `cargo run -p beans-lang-java -- <file.java>`.

## Testing Policy

Quality over quantity. Tests should verify behavior that matters, not chase coverage.

- Write tests for non-trivial logic: parsing, indexing, resolution. Skip boilerplate getters/constructors.
- Assert on actual values: names, FQNs, kinds. Do not just assert `is_some()`.
- One good test that parses a realistic Java file and checks five things is better than five trivial tests checking one thing each.
- `cargo test --workspace` must pass before any task is marked complete.

### Fixture Test Framework

The primary way to encode expected LSP behavior. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the full tutorial. Two operations:

- `.complete(|items| { ... })` - test completions at cursor, meaning what items appear when pressing cmd+space.
- `.resolve()` - test go-to-definition / hover at cursor, meaning what this symbol resolves to.

```rust
#[test]
fn service_dot_completion() {
    fixture()
        .file("com/example/Service.java", r#"
            package com.example;
            public class Service {
                public String process(int count) { return null; }
                private int internal;
            }
        "#)
        .file("com/example/App.java", r#"
            package com.example;
            public class App {
                public void run(Service svc) {
                    svc.<cur>
                }
            }
        "#)
        .complete(|items| {
            assert!(items.has("process", SymbolKind::Method));
            assert!(!items.has("internal", SymbolKind::Field));
        })
        .expected_failure("member completion not yet implemented");
}
```

## Communication

Be critical and insightful. If a solution is subpar, say so plainly and explain the tradeoff.

## Collaboration Norms

We are designing this project together as peers. Do not just defer to instructions; push back when you see issues.

- The project owner is not a Rust expert. If a preference would lead to non-idiomatic Rust, anti-patterns, or footguns, say so plainly and explain the tradeoff.
- If a decision is technically wrong or risky, challenge it even after a preference has been stated.
- If you already capitulated on something and realize you should not have, retract and re-make the case.
- Distinguish between "I prefer X for clarity" and "X happens to be familiar to me". When in doubt, ask.

## Architecture Principle: Library-First

Beans is a library that happens to ship an LSP server. The LSP is one consumer, not the center.

- Other consumers should be possible: CLI tools, batch analyzers, IDE plugins that bypass LSP, custom integrations.
- No core module depends on `beans-lsp`. The dependency graph terminates at `beans-lsp`; it is a leaf, not a hub.
- Anything that the LSP "knows about all languages" needs is a smell. Either lift it into a lower crate, or make it pluggable.
- When designing, ask: "could a CLI tool use this without the LSP?" If no, something is structured wrong.
