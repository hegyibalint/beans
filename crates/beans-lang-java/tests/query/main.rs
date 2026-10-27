#[path = "../support/mod.rs"]
mod support;
use support::Models;

mod resolution;
mod storage;

use beans_core::location::Location;
use beans_lang_java::{
    lowering::lower_into,
    model::nodes::NodeKind,
    query,
    semantics::resolution::{Context, JavaTypeCandidate, TypeCandidate, resolve},
};

#[test]
fn stored_models_can_be_resolved_using_the_engines_query() {
    let mut engine = Models::new();
    let source = Location::file("/src/app/Use.java");
    engine.store(
        lower_into("package app; class Target {} class Use { Target target; }"),
        source.clone(),
    );
    let file = query::file(&source, &engine.connection)
        .unwrap()
        .expect("expected the stored model");
    let (node_index, type_ref) = file
        .iter_nodes()
        .find_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) if field.name == "target" => {
                Some((entry.index, &field.declared_type))
            }
            _ => None,
        })
        .expect("expected target field");

    let ctx = Context::new(&file, &source, node_index, type_ref.value());
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(declaration)) = resolve(&ctx).unwrap()
    else {
        panic!("expected a Java declaration");
    };
    let target = query::declaration(&declaration, &engine.connection)
        .unwrap()
        .expect("expected a readable declaration handle");

    assert_eq!(target.name(), Some("Target"));
    assert_eq!(declaration.source(), &source);
}

#[test]
fn an_imported_type_resolves_to_another_stored_source_file() {
    let mut engine = Models::new();
    let use_source = Location::file("/src/p/Use.java");
    let target_source = Location::file("/src/q/Target.java");
    engine.store(
        lower_into("package p; import q.Target; class Use { Target field; }"),
        use_source.clone(),
    );
    engine.store(
        lower_into("package q; public class Target {}"),
        target_source.clone(),
    );
    let file = query::file(&use_source, &engine.connection)
        .unwrap()
        .unwrap();
    let field = file
        .iter_nodes()
        .find_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) => Some((entry.index, field)),
            _ => None,
        })
        .unwrap();
    let ctx = Context::new(&file, &use_source, field.0, field.1.declared_type.value())
        .with_connection(&engine.connection);

    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = resolve(&ctx).unwrap() else {
        panic!("expected a stored Java type");
    };
    assert_eq!(handle.source(), &target_source);
    assert_eq!(
        query::declaration(&handle, &engine.connection)
            .unwrap()
            .unwrap()
            .name(),
        Some("Target")
    );
}
