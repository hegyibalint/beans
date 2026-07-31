// What a unit can see is its own sources plus the units it depends on, which is
// what `Unit::depends_on` in the workspace model means. The edge points one way:
// a unit that is depended upon must not see back down it, and two units with no
// edge between them must not see each other at all.
//
// Every file here declares `package p`, so a package boundary cannot be what
// stops a reference; the unit boundary is the only thing left. That is not an
// exotic arrangement either, it is what a main and a test source set look like.
//
// `depends_on` is followed one hop. A chain of two would tell these tests
// nothing, because `a -> b` is all they declare.

use beans_acceptance::fixture::fixture;

#[test]
fn a_unit_sees_the_unit_it_depends_on() {
    fixture()
        .unit("a", &["a"], &[])
        .unit("b", &["b"], &["a"])
        .file("a/p/A.java", "package p; public class A {}")
        .file(
            "b/p/B.java",
            "package p; public class B { <cur:down>A field; }",
        )
        .analyze("b/p/B.java")
        .resolves_to("down", "p.A")
        .run();
}

#[test]
fn a_unit_cannot_see_the_unit_that_depends_on_it() {
    fixture()
        .unit("a", &["a"], &[])
        .unit("b", &["b"], &["a"])
        .file(
            "a/p/A.java",
            "package p; public class A { <cur:up>B field; }",
        )
        .file("b/p/B.java", "package p; public class B {}")
        .analyze("a/p/A.java")
        .does_not_resolve("up")
        .run();
}

#[test]
fn neither_unit_sees_the_other() {
    fixture()
        .unit("a", &["a"], &[])
        .unit("b", &["b"], &[])
        .file(
            "a/p/A.java",
            "package p; public class A { <cur:to_b>B field; }",
        )
        .file(
            "b/p/B.java",
            "package p; public class B { <cur:to_a>A field; }",
        )
        .analyze("a/p/A.java")
        .does_not_resolve("to_b")
        .analyze("b/p/B.java")
        .does_not_resolve("to_a")
        .run();
}

/// Shared code: `common` is declared by both units, so a file under it is
/// compiled by both and reaches what either one provides. That is the
/// permissive reading, and it is a false negative on purpose; the strict answer
/// analyses the file once per unit, which needs an analysis to know which unit
/// it is standing in.
#[test]
fn a_file_two_units_claim_reaches_what_either_one_provides() {
    fixture()
        .unit("a", &["a", "common"], &[])
        .unit("b", &["b", "common"], &[])
        .file("a/p/A.java", "package p; public class A {}")
        .file("b/p/B.java", "package p; public class B {}")
        .file(
            "common/p/Shared.java",
            "package p; public class Shared { <cur:to_a>A a; <cur:to_b>B b; }",
        )
        .analyze("common/p/Shared.java")
        .resolves_to("to_a", "p.A")
        .resolves_to("to_b", "p.B")
        .run();
}

/// The other direction: both units claim the tree, so both reach into it.
#[test]
fn every_unit_claiming_a_tree_reaches_into_it() {
    fixture()
        .unit("a", &["a", "common"], &[])
        .unit("b", &["b", "common"], &[])
        .file("common/p/Shared.java", "package p; public class Shared {}")
        .file(
            "a/p/A.java",
            "package p; public class A { <cur:from_a>Shared s; }",
        )
        .file(
            "b/p/B.java",
            "package p; public class B { <cur:from_b>Shared s; }",
        )
        .analyze("a/p/A.java")
        .resolves_to("from_a", "p.Shared")
        .analyze("b/p/B.java")
        .resolves_to("from_b", "p.Shared")
        .run();
}
