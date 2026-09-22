use beans_core_engine::Revision;
use beans_core_model::{names::Name, source::Source};
use beans_lang_java_model::{
    File,
    nodes::{NodeIndex, NodeKind},
};

use crate::{Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate, resolve};

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
    let source = Source::SourceFile {
        path: "Test.java".into(),
    };
    let ctx = Context::new(
        Revision::new(1),
        &source,
        file,
        field_index,
        &field.declared_type,
    );

    resolve(&ctx)
}

#[test]
fn a_type_parameter_across_a_static_nested_class_boundary_is_invalid() {
    let file = crate::lower_into("class Outer<T> { static class Nested { T target; } }");
    let outer = type_index(&file, &["Outer"]);

    let result = resolve_field(&file, "target");

    let Err(ResolutionFailure::IllegalTypeParameterUse(parameter)) = result else {
        panic!("expected an illegal type-parameter use, got {result:?}");
    };
    assert_eq!(parameter.owner().node_index(), outer);
    assert_eq!(parameter.parameter(&file).unwrap().name, "T");
}

#[test]
fn an_inner_class_can_use_a_type_parameter_of_its_static_enclosing_class() {
    let file = crate::lower_into(
        "class Outer<T> { static class Nested<U> { class Inner { U target; } } }",
    );
    let nested = type_index(&file, &["Outer", "Nested"]);

    let result = resolve_field(&file, "target").unwrap();

    let TypeCandidate::Java(JavaTypeCandidate::TypeParameter(parameter)) = result else {
        panic!("expected one Java type parameter");
    };
    assert_eq!(parameter.owner().node_index(), nested);
    assert_eq!(parameter.parameter(&file).unwrap().name, "U");
}

#[test]
fn a_second_static_boundary_blocks_an_intermediate_type_parameter() {
    let file = crate::lower_into(
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
    let file =
        crate::lower_into("class Outer { class Member {} static class Nested { Member target; } }");
    let member = type_index(&file, &["Outer", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = result else {
        panic!("expected one Java declaration");
    };
    assert_eq!(handle.node_index(), member);
}
