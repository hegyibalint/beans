use beans_lang_java::{
    lower_into,
    semantics::resolution::{JavaTypeCandidate, ResolutionFailure, TypeCandidate},
};

use super::{resolve_field, resolved_java_index, type_index};

#[test]
fn simple_name_resolves_to_a_member_of_the_enclosing_type() {
    let file = lower_into("class Outer { class Member {} Member value; }");
    let member = type_index(&file, &["Outer", "Member"]);

    let resolved = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(resolved), member);
}

#[test]
fn nearest_enclosing_type_wins() {
    let file =
        lower_into("class Outer { class Member {} class Inner { class Member {} Member value; } }");
    let inner_member = type_index(&file, &["Outer", "Inner", "Member"]);

    let resolved = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(resolved), inner_member);
}

#[test]
fn class_type_parameter_is_a_java_candidate() {
    let file = lower_into("class Outer<T> { T value; }");
    let outer = type_index(&file, &["Outer"]);

    let resolved = resolve_field(&file, "value").unwrap();

    let TypeCandidate::Java(JavaTypeCandidate::TypeParameter(handle)) = resolved else {
        panic!("expected one Java type parameter");
    };
    assert_eq!(handle.owner().node_index(), outer);
    assert_eq!(handle.parameter(&file).unwrap().name.value(), "T");
}

#[test]
fn declared_member_shadows_a_type_parameter_of_the_same_type() {
    let file = lower_into("class Outer<T> { class T {} T value; }");
    let member = type_index(&file, &["Outer", "T"]);

    let resolved = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(resolved), member);
}

#[test]
fn nearer_type_parameter_wins_over_an_outer_member() {
    let file = lower_into("class Outer { class T {} class Inner<T> { T value; } }");
    let inner = type_index(&file, &["Outer", "Inner"]);

    let resolved = resolve_field(&file, "value").unwrap();

    let TypeCandidate::Java(JavaTypeCandidate::TypeParameter(handle)) = resolved else {
        panic!("expected one Java type parameter");
    };
    assert_eq!(handle.owner().node_index(), inner);
}

#[test]
fn duplicate_type_parameters_are_ambiguous() {
    let file = lower_into("class Outer<T, T> { T value; }");

    let result = resolve_field(&file, "value");

    let Err(ResolutionFailure::Ambiguous(candidates)) = result else {
        panic!("expected ambiguity, got {result:?}");
    };
    assert_eq!(candidates.len(), 2);
    assert!(candidates.iter().all(|candidate| matches!(
        candidate,
        TypeCandidate::Java(JavaTypeCandidate::TypeParameter(_))
    )));
}

#[test]
fn qualified_suffixes_are_looked_up_in_the_selected_owner() {
    let file = lower_into("class Outer { class First { class Second {} } First.Second value; }");
    let second = type_index(&file, &["Outer", "First", "Second"]);

    let resolved = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(resolved), second);
}

#[test]
fn a_missing_suffix_does_not_retry_an_outer_prefix() {
    let file =
        lower_into("class Outer { class A { class B {} } class Inner { class A {} A.B value; } }");

    let result = resolve_field(&file, "value");

    assert_eq!(result, Err(ResolutionFailure::Partial));
}

#[test]
fn duplicate_members_are_ambiguous() {
    let file = lower_into("class Outer { class Member {} class Member {} Member value; }");

    let result = resolve_field(&file, "value");

    let Err(ResolutionFailure::Ambiguous(candidates)) = result else {
        panic!("expected ambiguity, got {result:?}");
    };
    assert_eq!(candidates.len(), 2);
    assert!(
        candidates
            .iter()
            .all(|candidate| matches!(candidate, TypeCandidate::Java(_)))
    );
}

#[test]
fn an_unknown_simple_name_is_not_found() {
    let file = lower_into("class Outer { Missing value; }");

    let result = resolve_field(&file, "value");

    assert_eq!(result, Err(ResolutionFailure::NotFound));
}
