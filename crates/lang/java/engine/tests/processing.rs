use beans_lang_java_engine::JavaEngine;
use beans_lang_java_model::declarations::Declaration;

#[test]
fn processing_returns_the_lowered_java_model() {
    let model = JavaEngine::process("class Example {}");
    let mut declarations = model.iter_declarations();
    let entry = declarations.next().expect("expected the class declaration");
    let Declaration::Type(declaration) = entry.declaration else {
        panic!("expected a type declaration");
    };

    assert_eq!(declaration.name.as_deref(), Some("Example"));
    assert!(declarations.next().is_none());
}
