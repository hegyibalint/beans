# Contributing to Beans

This guide is for anyone — human or agent — working on beans. Read it before opening a PR.

## Overview

Beans is a multi-language LSP for JVM languages (Java, Kotlin, Groovy, Scala, Clojure), written in Rust. The killer feature is cross-language navigation: one symbol index that all five languages parse into, so go-to-def, find-references, and rename work across language boundaries.

Beans is a **library** that happens to ship an LSP. `beans-lsp` is a leaf in the dependency graph — nothing imports it. CLIs, batch analyzers, and IDE plugins can sit on `beans-core` directly.

Start here:

- [`README.md`](README.md) — project overview and the killer feature.
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — symbol model, symbol table, crate structure, data flow.
- [`docs/adr/`](docs/adr/) — architecture decision records. Read these before proposing structural changes.

## Development setup

Requirements: **Rust edition 2024** (rustc 1.85+ required, toolchain pinned via Cargo).

```sh
cargo build --workspace
cargo test --workspace        # must pass before any task is marked complete
cargo run -p beans-lang-java -- path/to/File.java   # parser CLI
cargo run -p beans-lsp                              # LSP server on stdio
```

Useful subsets:

| Command | Purpose |
|---------|---------|
| `cargo test -p beans-test-java` | All Java spec tests. |
| `cargo test -p beans-test-java jls_8` | One JLS chapter. |
| `cargo test -p beans-test-java jls_7_5_1` | One JLS section. |
| `cargo check --workspace` | Fast compile check. |
| `cargo clippy --workspace` | Lints. |

## Project structure

```
beans-core/           Library: symbol model, symbol table, Language trait, resolution.
                      Languages live as feature-gated modules (ADR-0019).
beans-lang-java/      Java source parser (tree-sitter-java) + JavaLanguage impl.
beans-lsp/            LSP server. Leaf consumer of beans-core (ADR-0020).
beans-test-harness/   Fixture test framework. Language-agnostic, no language deps.
beans-test-java/      Java spec tests. Uses harness + JavaLanguage via prelude.
```

Two architectural rules to keep in mind:

- [ADR-0019](docs/adr/0019-single-core-crate-with-feature-gated-languages.md) — *"Beans is one library crate, `beans-core`, containing the graph engine, the JVM model, and all language modules as feature-gated submodules."*
- [ADR-0020](docs/adr/0020-lsp-is-a-leaf-consumer.md) — *"`beans-lsp` is a leaf in the dependency graph. Nothing depends on it."*

If a change starts pulling logic into `beans-lsp`, that's a smell. If a non-LSP consumer (CLI, batch analyzer) couldn't reach the same functionality, the code is in the wrong crate.

## Writing tests

The fixture framework is the primary way to encode expected LSP behavior. Each test sets up source files with cursor markers, then queries what the LSP should present.

### Test mentality

Write tests from the developer's perspective. The developer has a cursor somewhere in their code — what should the LSP offer?

Two operations, in priority order:

1. **Completions** — *"I typed `svc.` and pressed cmd+space. What items appear?"* → `.complete()`
2. **Resolution** — *"I clicked on `User`. Where does it jump? What does hover show?"* → `.resolve()`

Most tests should be **multi-file**: a declaring file and a consuming file with cursors. Single-file declaration-site tests have low value — they don't exercise resolution across files, which is where most real bugs live.

### Quick start: completion test

```rust
#[test]
fn dot_completion_on_service() {
    fixture()
        .file("com/example/Service.java", r#"
            package com.example;
            public class Service {
                public String process(int count) { return null; }
                public void shutdown() {}
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
        .complete_default(|items| {
            assert!(items.has("process", SymbolKind::Method));
            assert!(items.has("shutdown", SymbolKind::Method));
            assert!(!items.has("internal", SymbolKind::Field));
        })
        .expected_failure("member completion not yet implemented")
        .run();
}
```

### Quick start: resolution test

```rust
#[test]
fn import_resolves_to_class() {
    fixture()
        .file("com/example/model/User.java", r#"
            package com.example.model;
            public class User {}
        "#)
        .file("com/example/App.java", r#"
            package com.example;
            import com.example.model.User;
            public class App {
                private <cur>User user;
            }
        "#)
        .resolve()
            .resolves_to("com.example.model.User")
            .kind(SymbolKind::Class)
        .run();
}
```

### Cursor markers

Place `<cur>` or `<cur:name>` directly in the source. The harness strips the markers before parsing and records their positions.

| Marker | Usage |
|--------|-------|
| `<cur>` | Anonymous cursor. Use with `.complete_default(|items| ...)` or `.resolve()`. |
| `<cur:name>` | Named cursor. Use with `.complete("name", |items| ...)`, `.resolve("name")`, or `.assert_at("name")`. |

Cursor names must be unique across all files in a fixture.

```java
svc.<cur>                           // completion: what members are available?
private <cur:type>User user;        // resolution: where does User point?
```

### Completions

Test "what appears when the developer presses cmd+space here?"

```rust
.complete_default(|items| { ... })           // anonymous cursor
.complete("dot", |items| { ... })            // named cursor
```

`CompletionCandidates` (the value passed to the closure) is a thin
wrapper around the candidate list. Per ADR-0020 it carries only the
*neutral* shape — no LSP-formatted `detail` strings or wire-shaped
parameter lists. LSP-shaped formatting lives in `beans-lsp`.

`CompletionCandidates` query methods:

| Method | Returns | Purpose |
|--------|---------|---------|
| `has(name, kind)` | `bool` | Is this candidate offered? |
| `get(name, kind)` | `&CompletionCandidate` | Get the candidate (panics if missing). |
| `count(kind)` | `usize` | How many candidates of this kind? |
| `names(kind)` | `Vec<&str>` | Sorted names of all candidates of this kind. |
| `iter()` | iterator | Full access for edge cases. |

`CompletionCandidate` fields are all public — assert with `assert_eq!`:

| Field | Type |
|-------|------|
| `name` | `String` |
| `kind` | `SymbolKind` |
| `fqn` | `Fqn` |
| `node_id` | `NodeId` |

Tests assert on `name`, `kind`, and `fqn` for stable identity; the
`node_id` is the in-graph reference and is not stable across
rebuilds (per ADR-0007), so don't compare it across separate
fixture invocations.

Examples:

```rust
.complete_default(|items| {
    // Presence / absence
    assert!(items.has("getName", Method));
    assert!(!items.has("secret", Field));

    // Count
    assert_eq!(items.count(Method), 3);

    // All names of a kind
    assert_eq!(items.names(Method), &["close", "execute", "isOpen"]);

    // Deep inspection: identity-bearing fields only
    let exec = items.get("execute", Method);
    assert_eq!(exec.fqn.as_str(), "com.example.Service.execute");
})
```

### Resolution

Test "what does the LSP know about the symbol at this cursor?"

```rust
.resolve()                                   // anonymous cursor
    .resolves_to("com.example.Foo")
    .kind(SymbolKind::Class)
.run()

.resolve("field")                            // named cursor
    .hover_contains("String")
    .modifiers(vec![Modifier::Private])
.run()
```

Or for multi-cursor declaration-site checks, use `.assert_at("name")`:

```rust
.assert_at("class")
    .kind(SymbolKind::Class)
    .fqn("com.example.Dog")
    .children_include(&["name", "age", "getName", "getAge"])
.assert_at("getter")
    .kind(SymbolKind::Method)
    .signature_return("String")
.run();
```

Chainable assertions:

| Method | Purpose |
|--------|---------|
| `.kind(SymbolKind)` | Symbol kind. |
| `.fqn("...")` | Fully qualified name. |
| `.name("...")` | Simple name. |
| `.resolves_to("...")` | Go-to-definition target FQN. |
| `.hover_contains("...")` | Hover text substring. |
| `.signature_return("...")` | Method return type. |
| `.signature_params(&[("x", "int")])` | Method parameters. |
| `.modifiers(vec![...])` | Required modifiers. |
| `.parent_fqn("...")` | Enclosing symbol FQN. |
| `.children_include(&["..."])` | Child symbol names. |
| `.children_count(n)` | Exact child count. |

End the chain with `.run()`.

### Expected failure and skip

```rust
// Expected failure: runs the test, expects it to fail.
// If it unexpectedly passes, the test fails — telling you to promote it.
.complete_default(|items| { assert!(items.has("process", Method)); })
.expected_failure("member completion not yet implemented")

// Also works on resolution
.resolve("overload")
    .resolves_to("com.example.Foo.bar(int)")
    .expected_failure("overload resolution not yet correct")

// Skip: don't run, just log.
.resolve("diamond")
    .skip("diamond inference not implemented")
```

### File organization

Java spec tests are organized by JLS chapter, with nested modules per section:

```
beans-test-java/tests/
    prelude.rs                      # fixture() with JavaLanguage
    spec.rs                         # module root
    spec/
        jls04_types.rs              # Ch 4: Types, Values, Variables
        jls06_names.rs              # Ch 6: Names
        jls07_packages.rs           # Ch 7: Packages and Modules
        jls08_classes.rs            # Ch 8: Classes
        jls09_interfaces.rs         # Ch 9: Interfaces
        jls10_arrays.rs             # Ch 10: Arrays
        jls14_statements.rs         # Ch 14: Blocks, Statements, Patterns
        jls15_expressions.rs        # Ch 15: Expressions
```

Inside a chapter file, group tests by spec section:

```rust
mod jls_7_5_1_single_type_import {
    use super::*;

    #[test]
    fn basic() { ... }
}
```

This lets you run subsets: `cargo test -p beans-test-java jls_7` (chapter), `jls_7_5_1` (section).

## Spec test discipline

The spec test suite is the enforcement mechanism for beans' spec-driven posture. Four ADRs govern how it gets written. Read them in full before mass-authoring tests.

| ADR | One-line summary |
|-----|------------------|
| [ADR-0023](docs/adr/0023-mass-author-spec-tests-via-llm-agents-with-human-review.md) | *"Spec tests are mass-authored by LLM agents and reviewed by humans before merging."* |
| [ADR-0024](docs/adr/0024-tests-start-expected-failure-and-prefer-negative-spec-violations.md) | *"Every spec test starts marked `expected_failure` ... Tests prefer specific assertable facts over absence."* |
| [ADR-0025](docs/adr/0025-dual-mode-check-real-engine-vs-empty-engine.md) | *"The CI pipeline runs the test suite twice: once against the real engine, once against the empty engine. A test that passes in both modes is a trivial-passer."* |
| [ADR-0026](docs/adr/0026-per-test-opt-out-for-absence-dependent-tests.md) | *"The fixture framework supports a per-test marker that opts a test out of the dual-mode check ... Takes a required justification string."* |

Boiled down to four rules:

1. **Every spec test starts `expected_failure`.** No exceptions during the spec-authoring phase. The marker is a bookmark — *"this is what we owe the spec"* — not a permanent excuse. If the engine actually implements the feature, the unexpected pass becomes a "promote me" signal.

2. **Prefer negative spec violations over "valid code → no diagnostics" assertions.** Most spec rules are constraints. Their natural test form is a violation: feed the engine code that breaks the rule, assert it produces the specific diagnostic. A "valid code produces no diagnostics" test passes trivially against a stub engine that does nothing — it's the canonical trivial-passer.

3. **The dual-mode CI check catches trivial-passers automatically.** A Cargo feature flag toggles an "empty engine" that returns nothing for every query. CI runs the suite twice; tests that pass in both modes are flagged. Local `cargo test` runs only real-engine mode (speed of iteration matters more locally).

4. **Use the per-test opt-out for genuinely absence-dependent tests.** Some tests are about *absence itself* being the LSP-visible behavior (e.g., unresolved-import diagnostics). Mark these with `.absence_dependent("reason")`. The marker is rare by design — a spike in markers in a PR is a review prompt, not a free pass.

For each spec rule, ask: *"what is the most specific fact I can assert that would change if the engine stopped working?"* That is the assertion the test should make.

## Architecture decisions

Significant design choices live as ADRs in [`docs/adr/`](docs/adr/). The index is in [`docs/adr/README.md`](docs/adr/README.md).

ADRs describe **why**. Reference docs describe **what is**. Without ADRs, rationale lives in chat logs and PR descriptions, where it rots.

**Write a new ADR when:**

- You're making a structural decision that future contributors will want to revisit (crate boundaries, data flow, concurrency model, public API shape).
- You're picking one approach over plausible alternatives that should be documented.
- You're reversing an existing decision — write a new ADR with status `Superseded by ADR-XXXX`, leave the old file as historical record.

**Don't write an ADR when:**

- It's a minor implementation detail (which private function lives where, naming conventions). That belongs in code review.
- The decision is forced by external constraints with no real choice.
- The decision isn't expected to be revisited.

When in doubt, err on the side of writing one. Use the template in [`docs/adr/README.md`](docs/adr/README.md). The `Alternatives considered` section is not optional.

## Tracking work

Backlog items live in [`backlog/`](backlog/) as one file per item. The format is documented in `backlog/README.md`. Use the backlog for:

- Concrete tasks larger than a single PR.
- Spec coverage gaps (a section that needs tests authored).
- Known engine limitations that should be tracked but aren't blocking.

Don't use the backlog for trivial fixes — open a PR. Don't use it as a wishlist with no path to action.

## Submitting changes

1. Branch from `main`.
2. Write tests first (or alongside) for non-trivial logic. Quality over quantity — one good test that parses a realistic Java file and checks five things beats five trivial tests.
3. `cargo test --workspace` must pass before any task is marked complete.
4. Run `cargo clippy --workspace` and address warnings.
5. Open a PR. **Open it as a draft** until you're ready for review.
6. Reference any related ADRs in the PR description. If the change introduces a new architectural decision, write the ADR in the same PR.

### Collaboration norms

See [`CLAUDE.md`](CLAUDE.md) for the full collaboration norms. The headlines:

- **Be critical and insightful.** This is merit-based; speak up when something is subpar.
- **Push back on bad decisions.** Don't just defer to instructions — debate them.
- **No Rust footguns.** If a stated preference would lead to non-idiomatic Rust or anti-patterns, say so plainly and explain the tradeoff.

The goal is honest engineering disagreement leading to better designs, not capitulation.
