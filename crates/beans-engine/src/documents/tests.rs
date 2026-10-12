use beans_core::{
    features::HoverRequest,
    ranges::ByteRange,
    resource::{ResourceEntry, ResourceId, ResourceRoot},
    revision::Revision,
};

use crate::{
    Engine,
    storage::{Content, Model},
};

#[test]
fn processing_java_source_advances_the_revision_and_stores_its_model() {
    let mut engine = Engine::new();
    let source = ResourceId::file("/src/example/Example.java");

    let contents = "package example; public class Example {}";
    engine.process(source.clone(), contents);

    assert_eq!(engine.revision(), Revision::new(1));
    let entry = engine.storage.get(&source).unwrap();
    let Model::Java(file) = &entry.model else {
        panic!("expected a Java model");
    };
    assert_eq!(file.package_name.as_slice(), ["example"]);
    let Content::Stored(bytes) = entry.content.as_ref().unwrap();
    assert_eq!(bytes.as_slice(), contents.as_bytes());
}

#[test]
fn virtual_documents_keep_their_identity_through_replacement_and_removal() {
    let mut engine = Engine::new();
    let source = ResourceId::new(ResourceRoot::Virtual {
        provider: "test".into(),
        key: "untitled:Example.java".into(),
    });

    for (index, (contents, end)) in [
        ("class C extends First {}", 21),
        ("class C extends Second {}", 22),
    ]
    .into_iter()
    .enumerate()
    {
        if index == 0 {
            engine.document_open(source.clone(), "java", contents);
        } else {
            engine.document_edit(source.clone(), "java", contents);
        }
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
                .storage
                .get(&ResourceId::file("/Example.java"))
                .is_none()
        );
    }

    engine.document_close(source.clone());
    assert_eq!(engine.revision(), Revision::new(3));
    assert!(engine.storage.get(&source).is_none());
}

#[test]
fn selected_java_documents_do_not_require_a_java_filename() {
    let contents = "package example; class Example {}";
    for source in [
        ResourceId::file("/Example.txt"),
        ResourceId::new(ResourceRoot::Virtual {
            provider: "test".into(),
            key: "untitled:Untitled-1".into(),
        }),
    ] {
        let mut engine = Engine::new();
        engine.process(source.clone(), contents);
        assert_eq!(engine.revision(), Revision::default());

        engine.document_open(source.clone(), "java", contents);
        let entry = engine.storage.get(&source).unwrap();
        let Model::Java(file) = &entry.model else {
            panic!("expected a Java model");
        };
        assert_eq!(file.package_name.as_slice(), ["example"]);
        let Content::Stored(bytes) = entry.content.as_ref().unwrap();
        assert_eq!(bytes.as_slice(), contents.as_bytes());

        engine.document_close(source.clone());
        assert!(engine.storage.get(&source).is_none());
    }
}

#[test]
fn incomplete_java_is_published_with_its_original_content() {
    let mut engine = Engine::new();
    let source = ResourceId::file("/Example.java");
    let contents = "class Example { Missing field;";
    engine.document_open(source.clone(), "java", contents);

    let entry = engine.storage.get(&source).unwrap();
    assert!(matches!(entry.model, Model::Java(_)));
    let Content::Stored(bytes) = entry.content.as_ref().unwrap();
    assert_eq!(bytes.as_slice(), contents.as_bytes());
}

#[test]
fn unsupported_source_inputs_do_not_advance_the_revision() {
    let mut engine = Engine::new();
    let virtual_source = ResourceId::new(ResourceRoot::Virtual {
        provider: "test".into(),
        key: "Example.java".into(),
    });

    engine.process(ResourceId::file("/README.md"), "not Java");
    engine.process(ResourceId::file("/lib/Example.class"), "not Java source");
    engine.process(virtual_source.clone(), "class C extends T {}");
    engine.process(
        ResourceId::file("/sources.jar").entry(ResourceEntry("Example.java".into())),
        "class C extends T {}",
    );
    engine.document_open(virtual_source.clone(), "plaintext", "class C extends T {}");
    engine.document_edit(virtual_source.clone(), "plaintext", "class C extends T {}");

    assert_eq!(engine.revision(), Revision::default());
    assert!(engine.storage.get(&virtual_source).is_none());
}
