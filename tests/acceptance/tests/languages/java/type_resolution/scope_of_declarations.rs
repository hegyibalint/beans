// JLS §§6.3, 6.5.5.1, 8.2, and 9.2.

use beans_acceptance::fixture::fixture;

#[test]
fn top_level_scope_is_not_order_dependent() {
    fixture()
        .file(
            "p/Test.java",
            "package p; class Test { <cur:target>Later f; } class Later {}",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.Later")
        .run();
}

#[test]
fn same_package_top_level_type_is_in_scope() {
    fixture()
        .file("p/X.java", "package p; class X {}")
        .file("p/Test.java", "package p; class Test { <cur:target>X f; }")
        .analyze("p/Test.java")
        .resolves_to("target", "p.X")
        .run();
}

#[test]
fn member_type_scope_is_not_order_dependent() {
    fixture()
        .file(
            "p/Outer.java",
            "package p; class Outer { <cur:target>Inner f; class Inner {} }",
        )
        .analyze("p/Outer.java")
        .resolves_to("target", "p.Outer.Inner")
        .run();
}

#[test]
fn enclosing_member_type_is_in_scope_in_a_nested_type() {
    fixture()
        .file(
            "p/Outer.java",
            "package p; class Outer { class X {} class Inner { <cur:target>X f; } }",
        )
        .analyze("p/Outer.java")
        .resolves_to("target", "p.Outer.X")
        .run();
}

#[test]
fn superclass_member_type_is_inherited() {
    fixture()
        .file("p/Base.java", "package p; class Base { class X {} }")
        .file(
            "p/Sub.java",
            "package p; class Sub extends Base { <cur:target>X f; }",
        )
        .analyze("p/Sub.java")
        .resolves_to("target", "p.Base.X")
        .expected_failure("inherited member types are not resolved")
        .run();
}

#[test]
fn superinterface_member_type_is_inherited() {
    fixture()
        .file(
            "p/Types.java",
            "package p; interface Types { interface X {} }",
        )
        .file(
            "p/Test.java",
            "package p; class Test implements Types { <cur:target>X f; }",
        )
        .analyze("p/Test.java")
        .resolves_to("target", "p.Types.X")
        .expected_failure("inherited member types are not resolved")
        .run();
}

#[test]
fn local_type_is_not_in_scope_before_its_declaration() {
    fixture()
        .file("p/Test.java", "package p; class Local {} class Test { void m() { <cur:target>Local before; class Local {} } }")
        .analyze("p/Test.java")
        .resolves_to("target", "p.Local")
        .run();
}

#[test]
fn local_type_is_not_in_scope_after_its_block() {
    fixture()
        .file("p/Test.java", "package p; class Local {} class Test { void m() { { class Local {} } <cur:target>Local after; } }")
        .analyze("p/Test.java")
        .resolves_to("target", "p.Local")
        .run();
}

#[test]
fn class_type_parameter_cannot_be_used_in_a_static_context() {
    fixture()
        .file(
            "p/Box.java",
            "package p; class Box<T> { static <cur:target>T value; }",
        )
        .analyze("p/Box.java")
        .expect_at("target", "illegal-type-parameter-use")
        .expected_failure("type parameter use checks are not implemented")
        .run();
}

#[test]
fn class_type_parameter_cannot_cross_a_static_nested_type() {
    fixture()
        .file(
            "p/Box.java",
            "package p; class Box<T> { static class Nested { <cur:target>T value; } }",
        )
        .analyze("p/Box.java")
        .expect_at("target", "illegal-type-parameter-use")
        .expected_failure("type parameter use checks are not implemented")
        .run();
}
