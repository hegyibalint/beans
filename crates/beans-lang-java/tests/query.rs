use beans_core::{
    classpath::{Classpath, Unrestricted},
    origin::Origin,
};
use beans_core::{query_scope::QueryScope, revision::Revision};
use beans_lang_java::{
    engine::JavaEngine,
    lowering::lower_into,
    model::nodes::NodeKind,
    semantics::resolution::{
        Context, JavaTypeCandidate, TypeCandidate, query::ResolutionQuery, resolve,
    },
};
use beans_platform_jvm::engine::JvmEngine;

#[test]
fn stored_models_can_be_resolved_using_the_engines_query() {
    let mut engine = JavaEngine::default();
    let revision = Revision::new(3);
    let entry = engine.store(
        revision,
        lower_into("package app; class Target {} class Use { Target target; }"),
        Origin::uri("file:///src/app/Use.java"),
    );
    let classpath = Unrestricted;
    let query = engine.query(QueryScope::new(entry.revision, &classpath));

    assert_eq!(query.revision(), revision);
    assert!(std::ptr::eq(
        query.classpath(),
        &classpath as &dyn Classpath
    ));
    let file = query.file(&entry.key).expect("expected the stored model");
    let (node_index, type_ref) = file
        .iter_nodes()
        .find_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) if field.name == "target" => {
                Some((entry.index, &field.declared_type))
            }
            _ => None,
        })
        .expect("expected target field");

    let ctx = Context::new(revision, &entry.key, file, node_index, type_ref.value());
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(declaration)) = resolve(&ctx).unwrap()
    else {
        panic!("expected a Java declaration");
    };
    let target = query
        .declaration(&declaration)
        .expect("expected a readable declaration handle");

    assert_eq!(target.declaration.name(), Some("Target"));
    assert_eq!(target.source, &entry.key);
}

#[test]
fn an_imported_type_resolves_to_another_stored_source_file() {
    let mut engine = JavaEngine::default();
    let revision = Revision::new(1);
    let use_source = Origin::uri("file:///src/p/Use.java");
    let target_source = Origin::uri("file:///src/q/Target.java");
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
    let jvm = JvmEngine::default();
    let definitions = ResolutionQuery::new(&java, &jvm);
    let file = java.file(&use_source).unwrap();
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
        file,
        field.0,
        field.1.declared_type.value(),
    )
    .with_definitions(&definitions);

    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = resolve(&ctx).unwrap() else {
        panic!("expected a stored Java type");
    };
    assert_eq!(handle.source(), &target_source);
    assert_eq!(
        java.declaration(&handle).unwrap().declaration.name(),
        Some("Target")
    );
}
