//! Interop: Kotlin **producer**, Java **consumer**.
//!
//! Kotlin encodes nullability in its type system; Java does not. When a
//! Java call site consumes a Kotlin declaration, beans' shared JVM
//! projection should carry the Kotlin nullness across the boundary so
//! the Java vertical can flag an unguarded dereference of a value the
//! Kotlin side declared nullable.
//!
//! This is the canonical interop case from issue #7 and the first
//! resident of `tests/interop/kotlin_java/`. It is a placeholder until
//! the Kotlin vertical exists (backlog #021, `kotlin-parser-skeleton`):
//! the fixture has no `.kt` parser to dispatch to yet, so the test is
//! `#[ignore]`d rather than `expected_failure` — there is nothing to
//! fail against. Promote it (drop `#[ignore]`, switch to
//! `expected_failure`, then to a real pass) as the Kotlin vertical and
//! the nullness projection land.

use crate::prelude::fixture;

#[test]
#[ignore = "awaits Kotlin vertical + JVM nullness projection (backlog #021)"]
fn nullable_kotlin_return_flagged_at_java_dereference() {
    // Kotlin producer: `find` returns a nullable `Account?`.
    // Java consumer: dereferences the result without a null check.
    // Once interop lands, the Java use site should carry a
    // nullable-dereference diagnostic sourced from the Kotlin nullness.
    fixture()
        .file(
            "com/example/Accounts.kt",
            r#"
            package com.example
            class Account { fun balance(): Int = 0 }
            class Accounts { fun find(id: String): Account? = null }
            "#,
        )
        .file(
            "com/example/App.java",
            r#"
            package com.example;
            public class App {
                public int run(Accounts accounts) {
                    return accounts.find("a").balance();
                }
            }
            "#,
        )
        .diagnostics("com/example/App.java", |findings| {
            assert!(findings.has_code("nullable-dereference"));
        })
        .run();
}
