use beans_core_engine::Revision;
use beans_core_model::{classpath::{Classpath, ClasspathElement}, source::Source};
use beans_lang_java_engine::JavaEngine;
use beans_lang_java_model::declarations::Declaration;
use beans_lang_java_semantics::{
    lower_into,
    resolution::{ResolutionResult, Resolver},
};

#[test]
fn stored_models_can_be_resolved_using_the_engines_query() {
    let mut engine = JavaEngine::default();
    let revision = Revision::new(3);
    engine.store(
        revision,
        lower_into("package p; public class Example {}"),
        Source::SourceFile { path: "src/p/Example.java".into() },
    );
    let entry = engine.store(
        revision,
        lower_into("package app; import p.Example; class Use { Example target; }"),
        Source::SourceFile { path: "src/app/Use.java".into() },
    );
    let classpath = Classpath::new(vec![ClasspathElement::new("src".into(), [0; 32].into())]);
    let query = engine.query(entry.revision, &classpath);

    assert_eq!(query.revision(), revision);
    assert!(std::ptr::eq(query.classpath(), &classpath));
    let file = query.file(&entry.key).expect("expected the stored model");
    let (scope_index, type_ref) = file
        .iter_declarations()
        .find_map(|entry| match entry.declaration {
            Declaration::Field(field) if field.name == "target" => {
                Some((entry.scope_index, &field.declared_type))
            }
            _ => None,
        })
        .expect("expected target field");

    assert!(matches!(
        Resolver {}.resolve(file, scope_index, type_ref, &query),
        ResolutionResult::Resolved
    ));
}
