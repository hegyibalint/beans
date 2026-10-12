use beans_core::{features::HoverRequest, resource::ResourceId};

use crate::Engine;

#[test]
fn hover_only_requires_a_request() {
    let mut engine = Engine::new();
    let source = ResourceId::file("/src/Example.java");
    let contents = "class C extends Target {}";
    engine.document_open(source.clone(), "java", contents);

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
fn hover_reads_the_engines_storage_entries_after_replacement() {
    let mut engine = Engine::new();
    let source = ResourceId::file("/src/Example.java");
    for (contents, expected) in [
        ("class C extends Former {}", "Former"),
        ("class C extends Latest {}", "Latest"),
    ] {
        engine.document_edit(source.clone(), "java", contents);
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
    let source = ResourceId::file("/src/Missing.java");

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
