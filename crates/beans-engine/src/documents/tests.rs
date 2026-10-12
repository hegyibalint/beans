use std::path::Path;

use beans_core::{
    features::HoverRequest,
    ranges::ByteRange,
    resource::{ResourceEntry, ResourceId, ResourceRoot},
    revision::Revision,
};

use crate::Engine;

#[test]
fn accepts_only_supported_source_files() {
    let engine = Engine::new();

    assert!(engine.accept(Path::new("src/Example.java")));
    assert!(!engine.accept(Path::new("README.md")));
    assert!(!engine.accept(Path::new("lib/Example.class")));
}

#[test]
fn processing_java_source_advances_the_revision_and_stores_its_model() {
    let mut engine = Engine::new();
    let source = ResourceId::file("/src/example/Example.java");

    engine.process(source.clone(), "package example; public class Example {}");

    assert_eq!(engine.revision(), Revision::new(1));
    let file = engine.java.file(engine.revision(), &source).unwrap();
    assert_eq!(file.package_name.as_slice(), ["example"]);
}

#[test]
fn virtual_documents_keep_their_identity_through_replacement_and_removal() {
    let mut engine = Engine::new();
    let source = ResourceId::new(ResourceRoot::Virtual {
        provider: "test".into(),
        key: "untitled:Example.java".into(),
    });

    for (contents, end) in [
        ("class C extends First {}", 21),
        ("class C extends Second {}", 22),
    ] {
        engine.process_document(source.clone(), "java", contents);
        let response = engine
            .feature_hover(&HoverRequest {
                source: &source,
                contents,
                offset: 16,
            })
            .unwrap();
        assert_eq!(response.range(), ByteRange::new(16, end));
        assert!(
            engine
                .java
                .file(engine.revision(), &ResourceId::file("/Example.java"))
                .is_none()
        );
    }

    engine.close_document(source.clone());
    assert_eq!(engine.revision(), Revision::new(3));
    assert!(engine.java.file(engine.revision(), &source).is_none());
}

#[test]
fn unsupported_source_inputs_do_not_advance_the_revision() {
    let mut engine = Engine::new();
    let virtual_source = ResourceId::new(ResourceRoot::Virtual {
        provider: "test".into(),
        key: "Example.java".into(),
    });

    engine.process(ResourceId::file("/README.md"), "not Java");
    engine.process(virtual_source.clone(), "class C extends T {}");
    engine.process(
        ResourceId::file("/sources.jar").entry(ResourceEntry("Example.java".into())),
        "class C extends T {}",
    );
    engine.process_document(virtual_source.clone(), "plaintext", "class C extends T {}");

    assert_eq!(engine.revision(), Revision::default());
    assert!(
        engine
            .java
            .file(engine.revision(), &virtual_source)
            .is_none()
    );
}
