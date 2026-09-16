use beans_core_engine::Revision;
use beans_core_model::{
    classpath::{Classpath, ClasspathElement},
    source::Source,
};
use beans_lang_java_engine::JavaEngine;
use beans_lang_java_model::nodes::NodeKind;
use beans_lang_java_semantics::{
    lower_into,
    resolution::{ReferenceLocation, Resolution, ResolvedDeclaration, Resolver, ResolverContext},
};

#[test]
fn stored_models_can_be_resolved_using_the_engines_query() {
    let mut engine = JavaEngine::default();
    let revision = Revision::new(3);
    engine.store(
        revision,
        lower_into("package p; public class Example {}"),
        Source::SourceFile {
            path: "src/p/Example.java".into(),
        },
    );
    let entry = engine.store(
        revision,
        lower_into("package app; import p.Example; class Use { Example target; }"),
        Source::SourceFile {
            path: "src/app/Use.java".into(),
        },
    );
    let classpath = Classpath::new(vec![ClasspathElement::new("src".into(), [0; 32].into())]);
    let query = engine.query(entry.revision, &classpath);

    assert_eq!(query.revision(), revision);
    assert!(std::ptr::eq(query.classpath(), &classpath));
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

    let ctx = ResolverContext::new(
        &entry.key,
        node_index,
        type_ref,
        ReferenceLocation::Body,
        &query,
    );
    let results = Resolver {}.resolve(&ctx);
    let [Ok(Resolution::Resolved(resolved))] = results.as_slice() else {
        panic!("expected a resolved type: {results:?}");
    };
    let ResolvedDeclaration::Java { declaration } = &resolved.declaration else {
        panic!("expected a Java declaration");
    };
    let target = query
        .declaration(declaration)
        .expect("expected a readable declaration handle");
    assert_eq!(target.declaration.name.as_deref(), Some("Example"));
    assert_eq!(
        target.source,
        &Source::SourceFile {
            path: "src/p/Example.java".into()
        }
    );
}
