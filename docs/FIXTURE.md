# Fixture Test Framework

The fixture framework encodes expected LSP behavior as tests. Each test is a small Rust function that sets up source files with cursor markers, then asserts what the LSP should know at each cursor.

## Quick start

```rust
use beans_core::SymbolKind;
use beans_test_harness::fixture::Fixture;

#[test]
fn import_resolves_to_class() {
    Fixture::new()
        .file("com/example/model/User.java", r#"
            package com.example.model;
            public class User {}
        "#)
        .file("com/example/App.java", r#"
            package com.example;
            import com.example.model.User;
            public class App {
                private <cur:field>User user;
            }
        "#)
        .assert_at("field")
            .resolves_to("com.example.model.User")
            .kind(SymbolKind::Class)
        .run();
}
```

Run with `cargo test -p beans-test-harness`.

## Cursor markers

Place `<cur>` or `<cur:name>` anywhere in a source file. The harness strips them before parsing.

| Marker | Usage |
|--------|-------|
| `<cur>` | Anonymous cursor. One per file. Assert with `.assert_default()`. |
| `<cur:name>` | Named cursor. Multiple per file. Assert with `.assert_at("name")`. |

Names must be unique across all files in a fixture.

### Where to place cursors

Cursors mark a position in the source. Place them immediately before the identifier you want to query:

```java
private <cur:type>User user;       // cursor on "User"
public String <cur:method>getName() // cursor on "getName"
```

## Assertions

Chain assertions after `.assert_at("name")`:

```rust
.assert_at("name")
    .kind(SymbolKind::Class)              // symbol kind
    .fqn("com.example.Foo")              // fully qualified name
    .name("Foo")                          // simple name
    .resolves_to("com.example.Foo")       // go-to-definition target
    .hover_contains("class Foo")          // hover text substring
    .signature_return("String")           // method return type
    .signature_params(&[("x", "int")])    // method parameters
    .modifiers(vec![Modifier::Public])    // required modifiers
    .parent_fqn("com.example.Bar")       // enclosing symbol
    .children_include(&["field", "method"]) // child symbol names
    .children_count(3)                    // exact child count
```

All assertions are optional and combinable. Use only what matters for the test.

## Multi-file tests

Add multiple files with `.file()`. Cursors can appear in any file:

```rust
Fixture::new()
    .file("model/User.java", r#"
        package com.example.model;
        public class User { ... }
    "#)
    .file("service/UserService.java", r#"
        package com.example.service;
        import com.example.model.User;
        public class UserService {
            private <cur:ref>User user;
        }
    "#)
    .assert_at("ref")
        .resolves_to("com.example.model.User")
    .run();
```

## Multi-language tests

The fixture dispatches parsing per file extension. Java is registered by default. For cross-language tests, register additional languages:

```rust
Fixture::new()
    .with_language(KotlinLanguage)
    .file("com/example/Helper.kt", KOTLIN_SOURCE)
    .file("com/example/App.java", JAVA_SOURCE_WITH_CURSORS)
    .assert_at("helper_ref")
        .resolves_to("com.example.Helper")
    .run();
```

## Skip and expected failure

For features not yet implemented:

```rust
// Skip: don't run the assertion, just log it
.assert_at("diamond")
    .skip("diamond inference not implemented")
    .resolves_to("java.util.ArrayList")

// Expected failure: run it, but expect it to fail
// If it passes, the test fails — telling you to promote it
.assert_at("overload")
    .expected_failure("overload resolution not yet correct")
    .resolves_to("com.example.Foo.bar(int)")
```

## How it works

1. The harness strips `<cur>` markers and records their positions
2. Each file is parsed by the `Language` matching its extension
3. Symbols are inserted into a shared `SymbolTable`
4. For each assertion, the harness finds the word at the cursor position, resolves it through the symbol table, and checks the expected properties

## File organization

Tests live in `beans-test-harness/tests/spec.rs`. As the suite grows, split into modules:

```
beans-test-harness/tests/
    spec.rs          # or spec/mod.rs
    spec/imports.rs
    spec/generics.rs
    spec/resolution.rs
    ...
```
