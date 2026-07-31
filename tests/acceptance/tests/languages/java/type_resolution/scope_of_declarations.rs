// JLS §§6.3, 6.5.5.1, 8.2, and 9.2.
//
// What is in scope where is decided by `resolve_lexical_type_name`, so those
// cases live in `resolution/tests/lexical.rs`: a member type and a top level
// type are in scope above their own declaration, a local class is not, and a
// local class is out of scope again after its block.
//
// One of them was here and could not have caught anything.
// `local_type_is_not_in_scope_before_its_declaration` asserted `p.Local` against
// a file holding both a top level `Local` and a local class of that name, and
// `declaration_label` spells both of them `p.Local`. The unit test that replaced
// it found that resolution reaches the local class, which §6.3 forbids; see
// `lexical::a_local_type_is_wrongly_in_scope_before_it_is_declared`.

use beans_acceptance::fixture::fixture;

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
