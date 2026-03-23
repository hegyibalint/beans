# Beans

Beans is a multi-language LSP for JVM languages, built in Rust. A single shared index enables cross-language navigation, completion, and diagnostics — something no combination of separate LSPs can do.

**Target languages:**
- Java (current focus)
- Kotlin (stage 1)
- Groovy (stage 1)
- Scala (stage 2)
- Clojure (stage 3)

## Documentation

- [DIRECTION.md](DIRECTION.md) — Vision, motivation, user stories, phased roadmap
- [ARCHITECTURE.md](ARCHITECTURE.md) — Symbol model, symbol table, crate structure, data flow

## Development

```sh
cargo test --workspace   # run all tests
cargo run -p beans-lsp   # start the LSP server
```
