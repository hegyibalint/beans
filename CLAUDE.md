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
beans-test-harness/   # Fixture-driven test framework for encoding spec behavior
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

The primary way to encode expected LSP behavior. Tests use `beans-test-harness` with cursor markers in source files and a chainable Rust assertion API. See `docs/FIXTURE.md` for the full tutorial.

```rust
Fixture::new()
    .file("Foo.java", r#"
        package com.example;
        public class <cur:cls>Foo {}
    "#)
    .assert_at("cls")
        .kind(SymbolKind::Class)
        .fqn("com.example.Foo")
    .run();
```

The framework is language-agnostic — it dispatches parsing per file extension via the `Language` trait. Multi-language interop tests use `.with_language()` to register additional languages.
