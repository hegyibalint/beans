// JLS §6.6, from the resolution side: whether a type being out of reach changes
// what a name resolves to. It must not; §6.6 decides what may be used, and
// `accessibility.rs` one level up is where saying so belongs.
//
// The two passing cases cannot fail today, because nothing blocks resolution at
// all. They are here for when something does: each is a type that is accessible
// and must go on resolving, which is the half of the rule an over-eager access
// check would break first.

use beans_acceptance::fixture::fixture;

#[test]
fn package_access_top_level_type_is_accessible_in_same_package() {
    fixture()
        .file("p/X.java", "package p; class X {}")
        .file("p/Test.java", "package p; class Test { <cur:target>X f; }")
        .analyze("p/Test.java")
        .resolves_to("target", "p.X")
        .run();
}

#[test]
fn package_access_top_level_type_is_not_importable_from_other_package() {
    fixture()
        .file("q/X.java", "package q; class X {}")
        .file("p/Test.java", "package p; class Test { <cur:target>X f; }")
        .analyze("p/Test.java")
        .expect_at("target", "unresolvable-type")
        .expected_failure("type accessibility is not checked when resolving a simple name")
        .run();
}

#[test]
fn private_member_type_is_accessible_within_same_top_level_nest() {
    fixture()
        .file(
            "p/Outer.java",
            "package p; class Outer { private class X {} class Inner { <cur:target>X f; } }",
        )
        .analyze("p/Outer.java")
        .resolves_to("target", "p.Outer.X")
        .run();
}

#[test]
fn private_member_type_is_inaccessible_outside_top_level_nest() {
    fixture()
        .file(
            "q/Outer.java",
            "package q; public class Outer { private class X {} }",
        )
        .file(
            "p/Test.java",
            "package p; class Test { <cur:target>q.Outer.X f; }",
        )
        .analyze("p/Test.java")
        .expect_at("target", "inaccessible-type")
        .expected_failure("qualified type references are not resolved yet")
        .run();
}

#[test]
fn protected_member_type_is_inherited_by_cross_package_subclass() {
    fixture()
        .file(
            "q/Base.java",
            "package q; public class Base { protected static class X {} }",
        )
        .file(
            "p/Test.java",
            "package p; class Test extends q.Base { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "q.Base.X")
        .expected_failure("inherited member types are not resolved")
        .run();
}

#[test]
fn package_access_member_type_is_not_inherited_across_packages() {
    fixture()
        .file(
            "q/Base.java",
            "package q; public class Base { static class X {} }",
        )
        .file(
            "p/Test.java",
            "package p; class Test extends q.Base { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .expect_at("target", "unresolvable-type")
        .expected_failure("inherited member types are not resolved")
        .run();
}

#[test]
fn accessible_member_of_inaccessible_enclosing_type_is_inaccessible() {
    fixture()
        .file(
            "q/Hidden.java",
            "package q; class Hidden { public static class X {} }",
        )
        .file(
            "p/Test.java",
            "package p; class Test { <cur:target>q.Hidden.X f; }",
        )
        .analyze("p/Test.java")
        .expect_at("target", "inaccessible-type")
        .expected_failure("qualified type references are not resolved yet")
        .run();
}
