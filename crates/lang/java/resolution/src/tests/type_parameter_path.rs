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
fn a_qualified_type_parameter_uses_member_types_from_its_bound() {
    let file = crate::lower_into(
        "class Scope { class Bound { class Member {} } class Owner<T extends Bound> { T.Member value; } }",
    );
    let member = type_index(&file, &["Scope", "Bound", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn a_bound_is_resolved_in_the_type_parameter_declaration_context() {
    let file = crate::lower_into(
        "class Scope { class Bound { class Member {} } class Owner<T extends Bound> { class Bound {} T.Member value; } }",
    );
    let member = type_index(&file, &["Scope", "Bound", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn a_nested_type_parameter_bound_can_use_an_outer_type_parameter() {
    let file = crate::lower_into(
        "class Bound { class Member {} } class Outer<T extends Bound> { class Inner<U extends T> { U.Member value; } }",
    );
    let member = type_index(&file, &["Bound", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn every_intersection_bound_contributes_member_types() {
    let file = crate::lower_into(
        "class Base {} interface Contract { class Member {} } class Owner<T extends Base & Contract> { T.Member value; }",
    );
    let member = type_index(&file, &["Contract", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn a_selected_type_parameter_does_not_fall_back_after_a_missing_suffix() {
    let file = crate::lower_into(
        "class Scope { class T { class Member {} } class Owner<T> { T.Member value; } }",
    );

    let result = resolve_field(&file, "value");

    assert_eq!(result, Err(ResolutionFailure::Partial));
}

#[test]
fn circular_type_parameter_bounds_terminate() {
    let file = crate::lower_into("class Owner<T extends U, U extends T> { T.Member value; }");
    let owner = type_index(&file, &["Owner"]);

    let result = resolve_field(&file, "value");

    let Err(ResolutionFailure::CircularTypeParameterBound(parameter)) = result else {
        panic!("expected a circular-bound failure, got {result:?}");
    };
    assert_eq!(parameter.owner().node_index(), owner);
    assert_eq!(parameter.parameter(&file).unwrap().name, "T");
}

#[test]
fn an_illegal_type_parameter_use_is_validated_before_substitution() {
    let file = crate::lower_into(
        "class Bound { class Member {} } class Outer<T extends Bound> { static class Nested { T.Member value; } }",
    );
    let outer = type_index(&file, &["Outer"]);

    let result = resolve_field(&file, "value");

    let Err(ResolutionFailure::IllegalTypeParameterUse(parameter)) = result else {
        panic!("expected an illegal type-parameter use, got {result:?}");
    };
    assert_eq!(parameter.owner().node_index(), outer);
}
