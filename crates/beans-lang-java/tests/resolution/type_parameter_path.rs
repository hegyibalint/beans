use beans_lang_java::{lower_into, semantics::resolution::ResolutionFailure};

use super::{resolve_field, resolved_java_index, type_index};

#[test]
fn a_qualified_type_parameter_uses_member_types_from_its_bound() {
    let file = lower_into(
        "class Scope { class Bound { class Member {} } class Owner<T extends Bound> { T.Member value; } }",
    );
    let member = type_index(&file, &["Scope", "Bound", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn a_bound_is_resolved_in_the_type_parameter_declaration_context() {
    let file = lower_into(
        "class Scope { class Bound { class Member {} } class Owner<T extends Bound> { class Bound {} T.Member value; } }",
    );
    let member = type_index(&file, &["Scope", "Bound", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn a_nested_type_parameter_bound_can_use_an_outer_type_parameter() {
    let file = lower_into(
        "class Bound { class Member {} } class Outer<T extends Bound> { class Inner<U extends T> { U.Member value; } }",
    );
    let member = type_index(&file, &["Bound", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn every_intersection_bound_contributes_member_types() {
    let file = lower_into(
        "class Base {} interface Contract { class Member {} } class Owner<T extends Base & Contract> { T.Member value; }",
    );
    let member = type_index(&file, &["Contract", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn a_selected_type_parameter_does_not_fall_back_after_a_missing_suffix() {
    let file = lower_into(
        "class Scope { class T { class Member {} } class Owner<T> { T.Member value; } }",
    );

    let result = resolve_field(&file, "value");

    assert_eq!(result, Err(ResolutionFailure::Partial));
}

#[test]
fn circular_type_parameter_bounds_terminate() {
    let file = lower_into("class Owner<T extends U, U extends T> { T.Member value; }");
    let owner = type_index(&file, &["Owner"]);

    let result = resolve_field(&file, "value");

    let Err(ResolutionFailure::CircularTypeParameterBound(parameter)) = result else {
        panic!("expected a circular-bound failure, got {result:?}");
    };
    assert_eq!(parameter.owner().node_index(), owner);
    assert_eq!(parameter.parameter(&file).unwrap().name.value(), "T");
}

#[test]
fn an_illegal_type_parameter_use_is_validated_before_substitution() {
    let file = lower_into(
        "class Bound { class Member {} } class Outer<T extends Bound> { static class Nested { T.Member value; } }",
    );
    let outer = type_index(&file, &["Outer"]);

    let result = resolve_field(&file, "value");

    let Err(ResolutionFailure::IllegalTypeParameterUse(parameter)) = result else {
        panic!("expected an illegal type-parameter use, got {result:?}");
    };
    assert_eq!(parameter.owner().node_index(), outer);
}
