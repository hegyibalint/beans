# Fixture Test Framework

The fixture framework encodes expected LSP behavior as tests. Each test sets up source files with cursor markers, then queries what the LSP should present at each cursor.

## Test mentality

Write tests from the developer's perspective. The developer has a cursor somewhere in their code — what should the LSP offer?

Two operations, in priority order:

1. **Completions** — "I typed `svc.` and pressed cmd+space. What items appear?" → `.complete()`
2. **Resolution** — "I clicked on `User`. Where does it jump? What does hover show?" → `.resolve()`

Most tests should be **multi-file**: a declaring file and a consuming file with cursors. Single-file declaration-site tests have low value.

## Quick start

### Completion test

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
        .complete(|items| {
            assert!(items.has("process", SymbolKind::Method));
            assert!(items.has("shutdown", SymbolKind::Method));
            assert!(!items.has("internal", SymbolKind::Field));
        })
        .expected_failure("member completion not yet implemented");
}
```

### Resolution test

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

Run with `cargo test -p beans-test-java`.

## Architecture

```
beans-test-harness/     Framework library (language-agnostic)
beans-test-java/        Java spec tests + Java prelude
beans-test-kotlin/      Kotlin spec tests (future)
beans-test-interop/     Cross-language tests (future)
```

**`beans-test-harness`** provides `Fixture`, cursor markers, and both APIs (completions and resolution). No language dependencies.

**Per-language test crates** have a `prelude.rs`:

```rust
// beans-test-java/tests/prelude.rs
pub fn fixture() -> beans_test_harness::fixture::Fixture {
    beans_test_harness::fixture::Fixture::new()
        .with_language(beans_lang_java::JavaLanguage)
}
```

## Cursor markers

Place `<cur>` or `<cur:name>` in source files. The harness strips them before parsing.

| Marker | Usage |
|--------|-------|
| `<cur>` | Anonymous cursor. Use with `.complete(\|items\| ...)` or `.resolve()`. |
| `<cur:name>` | Named cursor. Use with `.complete("name", \|items\| ...)` or `.resolve("name")`. |

Names must be unique across all files in a fixture.

```java
svc.<cur>                           // completion: what members are available?
private <cur:type>User user;        // resolution: where does User point?
```

## Completions

Test "what appears when the developer presses cmd+space here?"

```rust
// Anonymous cursor
.complete(|items| { ... })

// Named cursor
.complete("dot", |items| { ... })

// With expected_failure
.complete(|items| { ... })
.expected_failure("reason")
```

### `CompletionItems` methods

| Method | Returns | Purpose |
|--------|---------|---------|
| `has(name, kind)` | `bool` | Is this item offered? |
| `get(name, kind)` | `&CompletionItem` | Get item (panics if missing) |
| `count(kind)` | `usize` | How many items of this kind? |
| `names(kind)` | `Vec<&str>` | Sorted names of all items of this kind |
| `iter()` | iterator | Full access for edge cases |

### `CompletionItem` fields

All public. Assert with `assert_eq!`.

| Field | Type |
|-------|------|
| `name` | `String` |
| `kind` | `SymbolKind` |
| `return_type` | `String` |
| `params` | `Vec<(String, String)>` |
| `modifiers` | `Vec<Modifier>` |
| `fqn` | `String` |
| `detail` | `String` |

### Examples

```rust
.complete(|items| {
    // Presence / absence
    assert!(items.has("getName", Method));
    assert!(!items.has("secret", Field));

    // Count
    assert_eq!(items.count(Method), 3);

    // All names of a kind
    assert_eq!(items.names(Method), &["close", "execute", "isOpen"]);

    // Deep inspection
    let exec = items.get("execute", Method);
    assert_eq!(exec.return_type, "void");
    assert_eq!(exec.params, &[("sql", "String")]);
})
```

## Resolution

Test "what does the LSP know about the symbol at this cursor?"

```rust
// Anonymous cursor
.resolve()
    .resolves_to("com.example.Foo")
    .kind(SymbolKind::Class)
.run()

// Named cursor
.resolve("field")
    .hover_contains("String")
    .modifiers(vec![Modifier::Private])
.run()
```

### Chainable assertions

| Method | Purpose |
|--------|---------|
| `.kind(SymbolKind)` | Symbol kind |
| `.fqn("...")` | Fully qualified name |
| `.name("...")` | Simple name |
| `.resolves_to("...")` | Go-to-definition target FQN |
| `.hover_contains("...")` | Hover text substring |
| `.signature_return("...")` | Method return type |
| `.signature_params(&[("x", "int")])` | Method parameters |
| `.modifiers(vec![...])` | Required modifiers |
| `.parent_fqn("...")` | Enclosing symbol FQN |
| `.children_include(&["..."])` | Child symbol names |
| `.children_count(n)` | Exact child count |

All optional and combinable. End with `.run()`.

## Multi-file tests

Most tests need at least two files — declaring and consuming:

```rust
fixture()
    .file("com/example/db/Connection.java", r#"
        package com.example.db;
        public class Connection {
            public void execute(String sql) {}
            public void close() {}
        }
    "#)
    .file("com/example/App.java", r#"
        package com.example;
        import com.example.db.Connection;
        public class App {
            public void query(Connection conn) {
                conn.<cur>
            }
        }
    "#)
    .complete(|items| {
        assert!(items.has("execute", SymbolKind::Method));
        assert!(items.has("close", SymbolKind::Method));
    })
    .expected_failure("cross-package member completion not yet implemented");
```

## Expected failure and skip

```rust
// Expected failure: runs the test, expects it to fail.
// If it unexpectedly passes → test fails, telling you to promote it.
.complete(|items| {
    assert!(items.has("process", Method));
})
.expected_failure("member completion not yet implemented")

// Also works on resolution
.resolve("overload")
    .resolves_to("com.example.Foo.bar(int)")
    .expected_failure("overload resolution not yet correct")

// Skip: don't run, just log
.resolve("diamond")
    .skip("diamond inference not implemented")
```

## How it works

1. The harness strips `<cur>` markers and records their positions
2. Each file is parsed by the `Language` matching its extension
3. Symbols are inserted into a shared `SymbolTable`
4. For **resolution**: resolves the word at cursor through the symbol table, checks properties
5. For **completions**: computes available items at cursor position, passes to closure

## File organization

Tests organized by JLS chapter, nested modules per section:

```
beans-test-java/tests/
    prelude.rs                      # fixture() with JavaLanguage
    spec.rs                         # module root
    spec/
        jls04_types.rs              # Ch 4: Types, Values, and Variables
        jls06_names.rs              # Ch 6: Names
        jls07_packages.rs           # Ch 7: Packages and Modules
        jls08_classes.rs            # Ch 8: Classes
        jls09_interfaces.rs         # Ch 9: Interfaces
        jls10_arrays.rs             # Ch 10: Arrays
        jls14_statements.rs         # Ch 14: Blocks, Statements, Patterns
        jls15_expressions.rs        # Ch 15: Expressions
```

```rust
mod jls_7_5_1_single_type_import {
    use super::*;

    #[test]
    fn basic() { ... }
}
```

Run subsets: `cargo test -p beans-test-java jls_7` (chapter), `jls_7_5_1` (section).
