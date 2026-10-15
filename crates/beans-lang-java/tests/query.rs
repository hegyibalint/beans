#[path = "../src/testing.rs"]
mod testing;
use testing::Models;

use beans_core::query_scope::QueryScope;
use beans_core::{
    classpath::{Classpath, Unrestricted},
    location::Location,
};
use beans_lang_java::{
    lowering::lower_into,
    model::nodes::NodeKind,
    semantics::resolution::{
        Context, JavaTypeCandidate, TypeCandidate, query::ResolutionQuery, resolve,
    },
};
use beans_platform_jvm::JvmPlatform;

#[test]
fn deletion_removes_the_model() {
    let mut models = Models::new();
    let source = Location::file("/Example.java");
    models.store(models.process("class Example {}"), source.clone());
    assert!(models.file(&source).is_some());
    models.remove(source.clone());
    assert!(models.file(&source).is_none());
}

#[test]
fn stored_models_can_be_resolved_using_the_engines_query() {
    let mut engine = Models::new();
    let source = Location::file("/src/app/Use.java");
    engine.store(
        lower_into("package app; class Target {} class Use { Target target; }"),
        source.clone(),
    );
    let classpath = Unrestricted;
    let query = engine.query(QueryScope::new(&classpath));

    assert!(std::ptr::eq(
        query.classpath(),
        &classpath as &dyn Classpath
    ));
    let file = query
        .file(&source)
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

    let ctx = Context::new(&source, &file, node_index, type_ref.value());
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(declaration)) = resolve(&ctx).unwrap()
    else {
        panic!("expected a Java declaration");
    };
    let target = query
        .declaration(&declaration)
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
    let classpath = Unrestricted;
    let java = engine.query(QueryScope::new(&classpath));
    let jvm = JvmPlatform::default();
    let definitions = ResolutionQuery::new(&java, &jvm);
    let file = java.file(&use_source).unwrap().unwrap();
    let field = file
        .iter_nodes()
        .find_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) => Some((entry.index, field)),
            _ => None,
        })
        .unwrap();
    let ctx = Context::new(&use_source, &file, field.0, field.1.declared_type.value())
        .with_definitions(&definitions);

    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = resolve(&ctx).unwrap() else {
        panic!("expected a stored Java type");
    };
    assert_eq!(handle.source(), &target_source);
    assert_eq!(
        java.declaration(&handle).unwrap().unwrap().name(),
        Some("Target")
    );
}
