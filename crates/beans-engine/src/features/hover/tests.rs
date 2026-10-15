use beans_core::{features::HoverRequest, location::Location, revision::Revision};

use crate::Engine;

#[test]
fn hover_only_requires_a_request() {
    let mut engine = Engine::new();
    let source = Location::file("/src/Example.java");
    let contents = "class C extends Target {}";
    let model = engine.java.process(contents);
    engine.revision = Revision::new(1);
    engine.java.store(engine.revision, model, source.clone());

    let response = engine
        .feature_hover(&HoverRequest {
            source: &source,
            contents,
            offset: contents.find("Target").unwrap(),
        })
        .unwrap();

    assert_eq!(response.contents(), "Target");
    let range = response.range();
    assert_eq!(&contents[range.start()..range.end()], "Target");
}

#[test]
fn hover_uses_the_engines_current_revision() {
    let mut engine = Engine::new();
    let source = Location::file("/src/Example.java");
    let versions = [
        (Revision::new(1), "class C extends Former {}", "Former"),
        (Revision::new(2), "class C extends Latest {}", "Latest"),
    ];
    for (revision, contents, _) in versions {
        let model = engine.java.process(contents);
        engine.java.store(revision, model, source.clone());
    }

    for (revision, contents, expected) in versions {
        engine.revision = revision;
        let response = engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: contents.find(expected).unwrap(),
            })
            .unwrap();
        assert_eq!(response.contents(), expected);
    }
}

#[test]
fn hover_of_an_unknown_resource_returns_none() {
    let engine = Engine::new();
    let source = Location::file("/src/Missing.java");

    assert!(
        engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents: "class C extends Target {}",
                offset: 16,
            })
            .is_none()
    );
}
