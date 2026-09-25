use crate::model::{
    File,
    nodes::{NodeIndex, NodeKind},
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
    let (field_index, field) = file
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
        field_index,
        field.declared_type.value(),
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
fn member_types_are_inherited_transitively() {
    let file = crate::lower_into(
        "class Scope { class Grandparent { class Member {} } class Parent extends Grandparent {} class Child extends Parent { Member target; } }",
    );
    let member = type_index(&file, &["Scope", "Grandparent", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn the_same_declaration_reached_through_a_diamond_is_not_ambiguous() {
    let file = crate::lower_into(
        "class Scope { interface Root { class Member {} } interface Left extends Root {} interface Right extends Root {} class Child implements Left, Right { Member target; } }",
    );
    let member = type_index(&file, &["Scope", "Root", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn distinct_declarations_inherited_from_different_interfaces_are_ambiguous() {
    let file = crate::lower_into(
        "class Scope { interface Left { class Member {} } interface Right { class Member {} } class Child implements Left, Right { Member target; } }",
    );

    let result = resolve_field(&file, "target");

    let Err(ResolutionFailure::Ambiguous(candidates)) = result else {
        panic!("expected ambiguity, got {result:?}");
    };
    let indices: Vec<_> = candidates
        .into_iter()
        .map(|candidate| match candidate {
            TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) => handle.node_index(),
            candidate => panic!("expected a Java declaration, got {candidate:?}"),
        })
        .collect();
    assert_eq!(
        indices,
        [
            type_index(&file, &["Scope", "Left", "Member"]),
            type_index(&file, &["Scope", "Right", "Member"]),
        ]
    );
}

#[test]
fn a_private_declaration_hides_its_ancestors_without_being_inherited() {
    let file = crate::lower_into(
        "class Scope { class Ancestor { class Member {} } class Parent extends Ancestor { private class Member {} } class Child extends Parent { Member target; } }",
    );

    let result = resolve_field(&file, "target");

    assert_eq!(result, Err(ResolutionFailure::NotFound));
}

#[test]
fn inherited_members_win_before_the_next_enclosing_type() {
    let file = crate::lower_into(
        "class Scope { class Member {} class Base { class Member {} } class Child extends Base { Member target; } }",
    );
    let inherited = type_index(&file, &["Scope", "Base", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), inherited);
}

#[test]
fn qualified_paths_include_inherited_member_types() {
    let file = crate::lower_into(
        "class Scope { class Base { class Member {} } class Child extends Base {} class Use { Child.Member target; } }",
    );
    let inherited = type_index(&file, &["Scope", "Base", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), inherited);
}

#[test]
fn an_unresolved_supertype_does_not_hide_later_interface_results() {
    let file = crate::lower_into(
        "class Scope { interface Contract { class Member {} } class Child extends Missing implements Contract { Member target; } }",
    );
    let member = type_index(&file, &["Scope", "Contract", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn circular_inheritance_terminates_without_producing_a_candidate() {
    let file = crate::lower_into(
        "class Scope { class First extends Second { Missing target; } class Second extends First {} }",
    );

    let result = resolve_field(&file, "target");

    assert_eq!(result, Err(ResolutionFailure::NotFound));
}
