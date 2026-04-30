---
status: pending
area: lsp
priority: low
---

# Feature-gate `Import` in beans-lsp to match the fixture's pattern

## Description

The LSP unconditionally imports
[`beans_core::languages::java::Import`](../beans-core/src/languages/java/syntax.rs)
in [`backend.rs`](../beans-lsp/src/backend.rs) and stores
`HashMap<PathBuf, Vec<Import>>` on `ServerState::file_imports`. This
treats Java's `Import` shape as the canonical "file imports" type for
all languages. Today that's correct because the LSP only handles Java
(its `Cargo.toml` declares `features = ["java"]`), but the LSP is
intended to support all five JVM languages (Kotlin, Scala, Groovy,
Clojure) when their parsers come online — and each language has its
own import syntax (Kotlin's `import` is similar to Java's; Scala has
selector clauses; Clojure has `(require ...)` / `(use ...)` forms).

The fixture harness took the principled path:

```rust
#[cfg(feature = "java")]
use beans_core::languages::java::Import;
#[cfg(not(feature = "java"))]
type Import = std::convert::Infallible;
```

This compiles without any language feature and forces the type system
to surface the gap when a non-Java language is added.

## Acceptance criteria

- `beans-lsp/src/backend.rs` cfg-gates the `Import` import the same
  way the fixture does.
- `ServerState::file_imports` is either:
  - cfg-gated (only present with `feature = "java"`), or
  - migrated to a per-language enum (e.g. `enum FileImports {
    Java(Vec<JavaImport>), ... }`) when the second language lands.
- `cargo build -p beans-lsp --no-default-features` clean (today this
  is moot because `beans-lsp/Cargo.toml` enables `features = ["java"]`
  on `beans-core`, but the pattern should be set up correctly).
- `cargo build --workspace` clean and all tests pass.

## Notes

This is forward-looking work. The LSP will hardcode `features =
["java"]` until at least one other language parser exists. The point
of the cfg-gate isn't immediate compatibility — it's the discipline
that the next person adding Kotlin support has to confront the
multi-language `Import` question deliberately rather than tripping
into it.
