use beans_acceptance::fixture::fixture;

// What a unit can see is its own sources plus the units it depends on, which is
// what `Unit::depends_on` in the workspace model means. The edge points one way:
// a unit that is depended upon must not see back down it, and two units with no
// edge between them must not see each other at all.
//
// Every file here declares `package p`, so a package boundary cannot be what
// stops a reference; the unit boundary is the only thing left. That is not an
// exotic arrangement either, it is what a main and a test source set look like.
//
// Nothing enforces any of this yet: the engine records the workspace and
// resolves unscoped, so every source sees the whole lake. The negative claims
// carry `expected_failure`. The positive ones pass, but for the wrong reason,
// and would keep passing with the dependency deleted. Both halves become honest
// the same day the negatives turn red.

mod one_way_dependency {
    use super::*;

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
            .expected_failure("resolution is unscoped; every source sees the whole lake")
            .run();
    }
}

mod unrelated_units {
    use super::*;

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
            .expected_failure("resolution is unscoped; every source sees the whole lake")
            .analyze("b/p/B.java")
            .does_not_resolve("to_a")
            .expected_failure("resolution is unscoped; every source sees the whole lake")
            .run();
    }
}
