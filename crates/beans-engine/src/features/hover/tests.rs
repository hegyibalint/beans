use beans_core::{features::HoverRequest, location::Location};

use crate::Engine;

#[test]
fn hover_only_requires_a_request() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();
    let source = Location::file("/src/Example.java");
    let contents = "class C extends Target {}";
    engine.process(source.clone(), contents).unwrap();

    let response = engine
        .feature_hover(&HoverRequest {
            source: &source,
            contents,
            offset: contents.find("Target").unwrap(),
        })
        .unwrap()
        .unwrap();

    assert_eq!(response.contents(), "Target");
    let range = response.range();
    assert_eq!(&contents[range.start()..range.end()], "Target");
}

#[test]
fn hover_uses_the_current_model() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();
    let source = Location::file("/src/Example.java");
    for (contents, expected) in [
        ("class C extends Former {}", "Former"),
        ("class C extends Latest {}", "Latest"),
    ] {
        engine.process(source.clone(), contents).unwrap();
        let response = engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: contents.find(expected).unwrap(),
            })
            .unwrap()
            .unwrap();
        assert_eq!(response.contents(), expected);
    }
}

#[test]
fn hover_of_an_unknown_resource_returns_none() {
    let root = tempfile::tempdir().unwrap();
    let engine = Engine::new(root.path()).unwrap();
    let source = Location::file("/src/Missing.java");

    assert!(
        engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents: "class C extends Target {}",
                offset: 16,
            })
            .unwrap()
            .is_none()
    );
}
