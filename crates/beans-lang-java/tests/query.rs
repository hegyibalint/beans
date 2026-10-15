#[path = "../src/testing.rs"]
mod testing;
use testing::Models;

use beans_core::{
    classpath::{Classpath, Unrestricted},
    location::Location,
};
use beans_core::{query_scope::QueryScope, revision::Revision};
use beans_lang_java::{
    lowering::lower_into,
    model::nodes::NodeKind,
    semantics::resolution::{
        Context, JavaTypeCandidate, TypeCandidate, query::ResolutionQuery, resolve,
    },
};
use beans_platform_jvm::JvmPlatform;

#[test]
fn deletion_keeps_older_models_readable() {
    let mut models = Models::new();
    let source = Location::file("/Example.java");
    models.store(
        Revision::new(1),
        models.process("class Example {}"),
        source.clone(),
    );
    models.remove(Revision::new(2), source.clone());
    assert!(models.file(Revision::new(1), &source).is_some());
    assert!(models.file(Revision::new(2), &source).is_none());
}

#[test]
fn stored_models_can_be_resolved_using_the_engines_query() {
    let mut engine = Models::new();
    let revision = Revision::new(3);
    let entry = engine.store(
        revision,
        lower_into("package app; class Target {} class Use { Target target; }"),
        Location::file("/src/app/Use.java"),
    );
    let classpath = Unrestricted;
    let query = engine.query(QueryScope::new(entry.revision, &classpath));

    assert_eq!(query.revision(), revision);
    assert!(std::ptr::eq(
        query.classpath(),
        &classpath as &dyn Classpath
    ));
    let file = query
        .file(&entry.key)
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

    let ctx = Context::new(revision, &entry.key, &file, node_index, type_ref.value());
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(declaration)) = resolve(&ctx).unwrap()
    else {
        panic!("expected a Java declaration");
    };
    let target = query
        .declaration(&declaration)
        .unwrap()
        .expect("expected a readable declaration handle");

    assert_eq!(target.name(), Some("Target"));
    assert_eq!(declaration.source(), &entry.key);
}

#[test]
fn an_imported_type_resolves_to_another_stored_source_file() {
    let mut engine = Models::new();
    let revision = Revision::new(1);
    let use_source = Location::file("/src/p/Use.java");
    let target_source = Location::file("/src/q/Target.java");
    engine.store(
        revision,
        lower_into("package p; import q.Target; class Use { Target field; }"),
        use_source.clone(),
    );
    engine.store(
        revision,
        lower_into("package q; public class Target {}"),
        target_source.clone(),
    );
    let classpath = Unrestricted;
    let java = engine.query(QueryScope::new(revision, &classpath));
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
    let ctx = Context::new(
        revision,
        &use_source,
        &file,
        field.0,
        field.1.declared_type.value(),
    )
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
