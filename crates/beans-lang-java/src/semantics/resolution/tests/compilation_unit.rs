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
fn a_top_level_type_is_visible_to_another_type_in_the_compilation_unit() {
    let file = crate::lower_into("class Target {} class Use { Target value; }");
    let target = type_index(&file, &["Target"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), target);
}

#[test]
fn a_top_level_type_can_refer_to_itself() {
    let file = crate::lower_into("class Target { Target value; }");
    let target = type_index(&file, &["Target"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), target);
}

#[test]
fn downward_lookup_continues_through_top_level_member_types() {
    let file = crate::lower_into("class Outer { class Inner {} } class Use { Outer.Inner value; }");
    let inner = type_index(&file, &["Outer", "Inner"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), inner);
}

#[test]
fn downward_lookup_includes_inherited_members_of_a_top_level_type() {
    let file = crate::lower_into(
        "class Base { class Member {} } class Child extends Base {} class Use { Child.Member value; }",
    );
    let member = type_index(&file, &["Base", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn duplicate_top_level_types_are_ambiguous() {
    let file = crate::lower_into("class Target {} class Target {} class Use { Target value; }");

    let result = resolve_field(&file, "value");

    let Err(ResolutionFailure::Ambiguous(candidates)) = result else {
        panic!("expected ambiguity, got {result:?}");
    };
    assert_eq!(candidates.len(), 2);
}
