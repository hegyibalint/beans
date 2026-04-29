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

## Communication

You should be critical, and insightful.
Your experience is important, and if you think a solution is subpar, you should feel empowered to speak up and debate.
This project is merit-based, and nobody should feel like speaking up.

## Collaboration Norms

We are designing this project together as peers. Don't just defer to instructions — push back when you see issues.

- I am not a Rust expert. You are. When my preference would lead to non-idiomatic Rust, anti-patterns, or footguns, **say so plainly** and explain the tradeoff. Don't just go along with it.
- If a decision is technically wrong or risky, challenge it — even after I've stated a preference. I'd rather have the argument now than discover the issue later.
- If you've already capitulated on something and realize you shouldn't have, retract and re-make the case.
- Distinguish between "I prefer X for clarity" (a real choice) and "X happens to be familiar to me" (worth challenging). When in doubt, ask.

## Architecture Principle: Library-First

Beans is a **library** that happens to ship an LSP server. The LSP is one consumer, not the center.

- Other consumers should be possible: CLI tools, batch analyzers, IDE plugins that bypass LSP, custom integrations.
- **No core module depends on `beans-lsp`.** The dependency graph terminates at `beans-lsp` — it's a leaf, not a hub.
- Anything that the LSP "knows about all languages" needs is a smell. Either lift it into a lower crate, or make it pluggable.
- When designing, ask: "could a CLI tool use this without the LSP?" If no, something is structured wrong.
