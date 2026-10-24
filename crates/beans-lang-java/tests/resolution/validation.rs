use beans_lang_java::{
    lower_into,
    semantics::resolution::{JavaTypeCandidate, ResolutionFailure, TypeCandidate},
};

use super::{resolve_field, type_index};

#[test]
fn a_type_parameter_across_a_static_nested_class_boundary_is_invalid() {
    let file = lower_into("class Outer<T> { static class Nested { T target; } }");
    let outer = type_index(&file, &["Outer"]);

    let result = resolve_field(&file, "target");

    let Err(ResolutionFailure::IllegalTypeParameterUse(parameter)) = result else {
        panic!("expected an illegal type-parameter use, got {result:?}");
    };
    assert_eq!(parameter.owner().node_index(), outer);
    assert_eq!(parameter.parameter(&file).unwrap().name.value(), "T");
}

#[test]
fn an_inner_class_can_use_a_type_parameter_of_its_static_enclosing_class() {
    let file =
        lower_into("class Outer<T> { static class Nested<U> { class Inner { U target; } } }");
    let nested = type_index(&file, &["Outer", "Nested"]);

    let result = resolve_field(&file, "target").unwrap();

    let TypeCandidate::Java(JavaTypeCandidate::TypeParameter(parameter)) = result else {
        panic!("expected one Java type parameter");
    };
    assert_eq!(parameter.owner().node_index(), nested);
    assert_eq!(parameter.parameter(&file).unwrap().name.value(), "U");
}

#[test]
fn a_second_static_boundary_blocks_an_intermediate_type_parameter() {
    let file = lower_into(
        "class Outer { static class First<T> { class Inner { static class Last { T target; } } } }",
    );
    let first = type_index(&file, &["Outer", "First"]);

    let result = resolve_field(&file, "target");

    let Err(ResolutionFailure::IllegalTypeParameterUse(parameter)) = result else {
        panic!("expected an illegal type-parameter use, got {result:?}");
    };
    assert_eq!(parameter.owner().node_index(), first);
}

#[test]
fn a_static_boundary_does_not_block_lexical_member_type_lookup() {
    let file = lower_into("class Outer { class Member {} static class Nested { Member target; } }");
    let member = type_index(&file, &["Outer", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = result else {
        panic!("expected one Java declaration");
    };
    assert_eq!(handle.node_index(), member);
}
