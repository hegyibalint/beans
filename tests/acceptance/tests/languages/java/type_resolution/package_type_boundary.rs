// Where a dotted name stops being a package and starts being a type.
// JLS §§6.5.4.1 and 6.5.4.2 walk it left to right and switch the first time the
// package to the left declares a type of that name; §7.1 forbids the two from
// colliding. A single-type import is the only surface where that walk is
// observable today, because a qualified type reference is not resolved yet.
//
// The walk itself is `resolve_canonical_name`, so its cases live in
// `resolution/tests/imports.rs`: where the package stops, nesting followed to
// the end, and a segment that must not be skipped. What is left here is the one
// thing only this level sees, two trees declaring one name into the same lake.

use beans_acceptance::fixture::fixture;

#[test]
fn a_type_prefix_leaves_no_way_back_to_a_package() {
    fixture()
        .file("p/Outer.java", "package p; public class Outer {}")
        .file(
            "p/Outer/Inner.java",
            "package p.Outer; public class Inner {}",
        )
        .file(
            "q/Test.java",
            "package q; import p.Outer.Inner; class Test { <cur:target>Inner f; }",
        )
        .analyze("q/Test.java")
        .expect_at("target", "unresolvable-type")
        .expected_failure("unresolvable type references produce no diagnostic yet")
        .run();
}

#[test]
fn a_prefix_that_names_nothing_is_unresolvable() {
    fixture()
        .file(
            "p/Test.java",
            "package p; import com.example.Missing; class Test { <cur:target>Missing f; }",
        )
        .analyze("p/Test.java")
        .expect_at("target", "unresolvable-type")
        .expected_failure("unresolvable type references produce no diagnostic yet")
        .run();
}

#[test]
fn the_unnamed_package_contributes_no_prefix() {
    fixture()
        .file(
            "Outer.java",
            "public class Outer { public static class Inner {} }",
        )
        .file("Test.java", "class Test { <cur:target>Outer.Inner f; }")
        .analyze("Test.java")
        .resolves_to("target", "Outer.Inner")
        .expected_failure("qualified type references are not resolved yet")
        .run();
}

// Both declare `p.B`, so the label cannot tell them apart even though the
// targets are distinct. Twice the same spelling is the honest expectation.
// Both declare `p.B`, so the label cannot tell them apart even though the
// targets are distinct. Twice the same spelling is the honest expectation.
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
