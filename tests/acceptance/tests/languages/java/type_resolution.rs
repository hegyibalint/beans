// Type resolution is assembled from rules across the JLS, and each rule is
// decided in the `crates/lang-java/src/resolution.rs`; that is where its cases
// live, in the `resolution/tests/`. What is left for this level is one test per
// rule, saying the answer reaches a user of Beans.
//
// So there is one case per stage of `resolve_type_name` that we have built, one
// for the order between two stages, and one for the answer being ambiguous. The
// stages we have not built get nothing here, because a test whose loser does not
// exist passes without proving anything; see the `TODO.md`.

use beans_acceptance::fixture::fixture;

/// Stage 2, §7.5.1.
#[test]
fn single_type_import_provides_simple_name() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.X; class Test { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.X")
        .run();
}

/// Stage 3, §6.3.
#[test]
fn same_package_top_level_type_is_in_scope() {
    fixture()
        .file("p/X.java", "package p; class X {}")
        .file("p/Test.java", "package p; class Test { <cur:target>X f; }")
        .analyze("p/Test.java")
        .resolves_to("target", "p.X")
        .run();
}

/// Stage 4, §7.3. A JDK is where `java.lang` comes from, and a class file has no
/// declaration to navigate to, so the package is declared here instead; the
/// runtime image reaching a user is the `engine/jdk.rs`.
#[test]
fn java_lang_is_in_scope_without_being_imported() {
    fixture()
        .file(
            "java/lang/Legend.java",
            "package java.lang; public class Legend {}",
        )
        .file(
            "p/Test.java",
            "package p; class Test { <cur:target>Legend f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "java.lang.Legend")
        .run();
}

/// Stage 1 over stage 2, §6.4.1. The order itself is settled in the
/// `resolution/tests/staging.rs`, where each case also removes the winner.
#[test]
fn member_type_shadows_single_type_import() {
    fixture()
        .file("q/X.java", "package q; public class X {}")
        .file(
            "p/Test.java",
            "package p; import q.X; class Test { class X {} <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.Test.X")
        .run();
}

/// Two trees declaring one name into one lake, which only this level assembles.
/// Both come back as `p.B`, so the expectation cannot say which pair it got; the
/// `TODO.md` carries what would fix that.
#[test]
fn one_name_declared_in_two_trees_is_ambiguous() {
    fixture()
        .file("app/p/B.java", "package p; public class B {}")
        .file("lib/p/B.java", "package p; public class B {}")
        .file(
            "q/Test.java",
            "package q; import p.B; class Test { <cur:target>B f; }",
        )
        .analyze("q/Test.java")
        .ambiguous_between("target", &["p.B", "p.B"])
        .run();
}

/// JLS §6.5.5.1 requires the type declaration to be in scope, while §7.3 lets
/// the host decide which compilation units are observable. The declaration is
/// indexed, but the main unit cannot observe its test-only source.
#[test]
fn a_known_type_outside_the_unit_scope_is_reported() {
    fixture()
        .unit("main", &["main"], &[])
        .unit("test", &["test"], &[])
        .file(
            "test/p/TestOnly.java",
            "package p; public class TestOnly { public int open; }",
        )
        .file(
            "main/p/Main.java",
            "package p; public class Main {
                void use(<cur:unavailable>TestOnly target) {
                    int value = target.<cur:member>open;
                }
            }",
        )
        .analyze("main/p/Main.java")
        .does_not_resolve("unavailable")
        .expect_at("unavailable", "type-outside-scope")
        .expect_no("inaccessible-member")
        .run();
}
