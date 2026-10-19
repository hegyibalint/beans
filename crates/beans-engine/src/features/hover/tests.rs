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

    assert_eq!(response.contents().plaintext(), "Target");
    let range = response.range();
    assert_eq!(&contents[range.start()..range.end()], "Target");
}

#[test]
fn resolved_hover_relativizes_workspace_sources() {
    let root = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(root.path()).unwrap();
    let contents = "package demo; class C { C value; }";
    let source = Location::file(root.path().join("src/demo/C.java"));
    engine.process(source.clone(), contents).unwrap();

    let response = engine
        .feature_hover(&HoverRequest {
            source: &source,
            contents,
            offset: contents.find("C value").unwrap(),
        })
        .unwrap()
        .unwrap();

    assert_eq!(
        response.contents().plaintext(),
        "Type: demo.C\nLocated in: src/demo/C.java"
    );
    assert_eq!(
        response.contents().markdown(),
        Some("**Type:** ` demo.C `\n\n**Located in:** ` src/demo/C.java `")
    );
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
        assert_eq!(response.contents().plaintext(), expected);
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
