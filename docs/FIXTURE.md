# Fixture Test Framework

The fixture framework encodes expected LSP behavior as tests. Each test is a small Rust function that sets up source files with cursor markers, then asserts what the LSP should know at each cursor.

## Quick start

```rust
mod prelude;
use prelude::fixture;
use beans_core::SymbolKind;

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
                private <cur:field>User user;
            }
        "#)
        .assert_at("field")
            .resolves_to("com.example.model.User")
            .kind(SymbolKind::Class)
        .run();
}
```

Run with `cargo test -p beans-test-java`.

## Architecture

The test infrastructure is split into layers:

```
beans-test-harness/     Framework library (language-agnostic)
beans-test-java/        Java spec tests + Java prelude
beans-test-kotlin/      Kotlin spec tests + Kotlin prelude (future)
beans-test-interop/     Cross-language tests (future)
```

**`beans-test-harness`** provides `Fixture`, cursor marker stripping, and the assertion API. It has no language dependencies — it doesn't know about Java, Kotlin, or any other language.

**Per-language test crates** each have a `prelude.rs` that creates a `Fixture` with the right language(s) registered:

```rust
// beans-test-java/tests/prelude.rs
use beans_test_harness::fixture::Fixture;
use beans_lang_java::JavaLanguage;

pub fn fixture() -> Fixture {
    Fixture::new()
        .with_language(JavaLanguage)
}
```

Test files import the prelude and call `fixture()` — no registration boilerplate:

```rust
mod prelude;
use prelude::fixture;

#[test]
fn my_test() {
    fixture()
        .file("Foo.java", src)
        .assert_at("x").kind(SymbolKind::Class)
        .run();
}
```

When a new language is added, it gets its own test crate with its own prelude. Existing tests don't change.

**`beans-test-interop/`** (future) will depend on all language crates and test cross-language scenarios — e.g., Java code referencing a Kotlin class. Its prelude registers every language. This is the only crate that needs all language dependencies, which is appropriate since cross-language testing inherently requires them.

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
fixture()
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

Each language has its own test crate. As the suite grows, split into modules:

```
beans-test-java/tests/
    prelude.rs           # fixture() with JavaLanguage
    spec.rs              # or spec/mod.rs
    spec/imports.rs
    spec/generics.rs
    spec/resolution.rs
    ...
```
