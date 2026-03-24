# Beans

Beans is a multi-language LSP for JVM languages (Java, Kotlin, Groovy, Scala, Clojure). See DIRECTION.md for vision and roadmap, ARCHITECTURE.md for technical design.

## Key Documents

- `DIRECTION.md` — Vision, motivation, user stories, phased roadmap
- `ARCHITECTURE.md` — Symbol model, symbol table, crate structure, data flow
- `docs/FIXTURE.md` — Fixture test framework tutorial

## Project Structure

```
beans-core/           # Shared JVM symbol model, symbol table, Language trait, resolution
beans-lang-java/      # Java source parser (tree-sitter-java) + JavaLanguage impl
beans-lsp/            # LSP server (go-to-def, hover, references, document symbols)
beans-test-harness/   # Fixture test framework (language-agnostic, no language deps)
beans-test-java/      # Java spec tests (uses harness + JavaLanguage via prelude)
```

## Development

- Language: Rust (edition 2024)
- Run tests: `cargo test`
- Run Java parser CLI: `cargo run -p beans-lang-java -- <file.java>`

## Testing Policy

Quality over quantity. Tests should verify behavior that matters, not chase coverage.

- Write tests for non-trivial logic — parsing, indexing, resolution. Skip boilerplate getters/constructors.
- Assert on actual values (names, FQNs, kinds), not just `is_some()`.
- One good test that parses a realistic Java file and checks 5 things > five trivial tests checking one thing each.
- `cargo test --workspace` must pass before any task is marked complete.

### Fixture Test Framework

The primary way to encode expected LSP behavior. See `docs/FIXTURE.md` for the full tutorial. Two operations:

- **`.complete(|items| { ... })`** — test completions at cursor (what items appear when pressing cmd+space)
- **`.resolve()`** — test go-to-definition / hover at cursor (what does this symbol resolve to)

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
