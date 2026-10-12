use super::*;
use beans_core::resource::ResourceRoot;

#[test]
fn accept_recognizes_java_source_paths() {
    let language = JavaLanguage::new();

    assert!(language.accept(Path::new("src/example/Example.java")));
    assert!(!language.accept(Path::new("README.md")));
    assert!(!language.accept(Path::new("Example.class")));
}

#[test]
fn process_returns_an_owned_model_for_selected_java_input() {
    let language = JavaLanguage::new();
    let source = ResourceId::file("/Example.java");
    let file = language
        .process(&source, "package example; class Example {}")
        .unwrap();

    assert_eq!(file.package_name.as_slice(), ["example"]);
}

#[test]
fn selected_java_input_does_not_require_a_java_filename() {
    let language = JavaLanguage::new();
    for source in [
        ResourceId::file("/Example.txt"),
        ResourceId::new(ResourceRoot::Virtual {
            provider: "test".into(),
            key: "untitled:Untitled-1".into(),
        }),
    ] {
        let file = language
            .process(&source, "package example; class Example {}")
            .unwrap();
        assert_eq!(file.package_name.as_slice(), ["example"]);
    }
}

#[test]
fn incomplete_java_input_still_produces_a_model() {
    let language = JavaLanguage::new();
    let source = ResourceId::file("/Example.java");

    assert!(language.process(&source, "").is_some());
    assert!(
        language
            .process(&source, "class Example { Missing field;")
            .is_some()
    );
}
