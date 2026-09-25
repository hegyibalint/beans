use crate::model::{
    File,
    nodes::{NodeIndex, NodeKind},
    references::TypeRef,
};
use beans_core::engine::Revision;
use beans_core::model::{names::Name, source::Source};

use crate::semantics::resolution::{
    Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate, resolve,
};

fn type_index(file: &File, components: &[&str]) -> NodeIndex {
    let name = Name::new(
        components
            .iter()
            .map(|component| (*component).to_owned())
            .collect(),
    );
    let matches = file.find_type(&name);
    assert_eq!(matches.len(), 1);
    matches[0].0
}

fn resolve_field(file: &File, field_name: &str) -> Result<TypeCandidate, ResolutionFailure> {
    let field = file
        .iter_nodes()
        .find_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) if field.name == field_name => Some((entry.index, field)),
            _ => None,
        })
        .expect("expected field");
    let source = Source::uri("file:///Test.java");
    let ctx = Context::new(
        Revision::new(1),
        &source,
        file,
        field.0,
        field.1.declared_type.value(),
    );

    resolve(&ctx)
}

fn resolved_java_index(result: TypeCandidate) -> NodeIndex {
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = result else {
        panic!("expected one Java declaration");
    };
    handle.node_index()
}

#[test]
fn simple_name_resolves_to_a_member_of_the_enclosing_type() {
    let file = crate::lower_into("class Outer { class Member {} Member value; }");
    let member = type_index(&file, &["Outer", "Member"]);

    let resolved = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(resolved), member);
}

#[test]
fn nearest_enclosing_type_wins() {
    let file = crate::lower_into(
        "class Outer { class Member {} class Inner { class Member {} Member value; } }",
    );
    let inner_member = type_index(&file, &["Outer", "Inner", "Member"]);

    let resolved = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(resolved), inner_member);
}

#[test]
fn class_type_parameter_is_a_java_candidate() {
    let file = crate::lower_into("class Outer<T> { T value; }");
    let outer = type_index(&file, &["Outer"]);

    let resolved = resolve_field(&file, "value").unwrap();

    let TypeCandidate::Java(JavaTypeCandidate::TypeParameter(handle)) = resolved else {
        panic!("expected one Java type parameter");
    };
    assert_eq!(handle.owner().node_index(), outer);
    assert_eq!(handle.parameter(&file).unwrap().name, "T");
}

#[test]
fn declared_member_shadows_a_type_parameter_of_the_same_type() {
    let file = crate::lower_into("class Outer<T> { class T {} T value; }");
    let member = type_index(&file, &["Outer", "T"]);

    let resolved = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(resolved), member);
}

#[test]
fn nearer_type_parameter_wins_over_an_outer_member() {
    let file = crate::lower_into("class Outer { class T {} class Inner<T> { T value; } }");
    let inner = type_index(&file, &["Outer", "Inner"]);

    let resolved = resolve_field(&file, "value").unwrap();

    let TypeCandidate::Java(JavaTypeCandidate::TypeParameter(handle)) = resolved else {
        panic!("expected one Java type parameter");
    };
    assert_eq!(handle.owner().node_index(), inner);
}

#[test]
fn duplicate_type_parameters_are_ambiguous() {
    let file = crate::lower_into("class Outer<T, T> { T value; }");

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
    let file =
        crate::lower_into("class Outer { class First { class Second {} } First.Second value; }");
    let second = type_index(&file, &["Outer", "First", "Second"]);

    let resolved = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(resolved), second);
}

#[test]
fn a_missing_suffix_does_not_retry_an_outer_prefix() {
    let file = crate::lower_into(
        "class Outer { class A { class B {} } class Inner { class A {} A.B value; } }",
    );

    let result = resolve_field(&file, "value");

    assert_eq!(result, Err(ResolutionFailure::Partial));
}

#[test]
fn duplicate_members_are_ambiguous() {
    let file = crate::lower_into("class Outer { class Member {} class Member {} Member value; }");

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
    let file = crate::lower_into("class Outer { Missing value; }");

    let result = resolve_field(&file, "value");

    assert_eq!(result, Err(ResolutionFailure::NotFound));
}

#[test]
fn a_non_named_reference_is_invalid() {
    let file = crate::lower_into("class Outer { int value; }");

    let result = resolve_field(&file, "value");

    assert_eq!(result, Err(ResolutionFailure::InvalidTypeRef));
}

#[test]
fn an_empty_named_reference_is_invalid() {
    let file = crate::lower_into("class Outer {}");
    let source = Source::uri("file:///Test.java");
    let type_ref = TypeRef::Named {
        segments: Vec::new(),
    };
    let ctx = Context::new(
        Revision::new(1),
        &source,
        &file,
        File::ROOT_NODE_ID,
        &type_ref,
    );

    assert_eq!(resolve(&ctx), Err(ResolutionFailure::InvalidTypeRef));
}
