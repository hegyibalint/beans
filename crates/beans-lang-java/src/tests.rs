use super::*;
use beans_core::resource::ResourceRoot;

#[test]
fn accept_recognizes_java_source_paths() {
    assert!(accept(Path::new("src/example/Example.java")));
    assert!(!accept(Path::new("README.md")));
    assert!(!accept(Path::new("Example.class")));
}

#[test]
fn process_returns_an_owned_model_for_selected_java_input() {
    let source = ResourceId::file("/Example.java");
    let file = process(&source, "package example; class Example {}").unwrap();

    assert_eq!(file.package_name.as_slice(), ["example"]);
}

#[test]
fn selected_java_input_does_not_require_a_java_filename() {
    for source in [
        ResourceId::file("/Example.txt"),
        ResourceId::new(ResourceRoot::Virtual {
            provider: "test".into(),
            key: "untitled:Untitled-1".into(),
        }),
    ] {
        let file = process(&source, "package example; class Example {}").unwrap();
        assert_eq!(file.package_name.as_slice(), ["example"]);
    }
}

#[test]
fn incomplete_java_input_still_produces_a_model() {
    let source = ResourceId::file("/Example.java");

    assert!(process(&source, "").is_some());
    assert!(process(&source, "class Example { Missing field;").is_some());
}
