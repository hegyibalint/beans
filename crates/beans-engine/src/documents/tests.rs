use beans_core::{
    features::HoverRequest,
    ranges::ByteRange,
    resource::{ResourceEntry, ResourceId, ResourceRoot},
    revision::Revision,
};

use crate::{
    Engine,
    documents::{Content, Documents, Entry, Model},
};

#[test]
fn document_entries_keep_the_model_and_retained_input_together() {
    let mut documents = Documents::new();
    let source = ResourceId::file("/src/example/Example.java");
    let contents = "package example; class Example {}";
    let model = beans_lang_java::engine::JavaLanguage::new().process(contents);

    assert!(
        documents
            .insert(
                source.clone(),
                Entry {
                    model: Model::Java(model),
                    content: Some(Content::Stored(contents.as_bytes().to_vec())),
                },
            )
            .is_none()
    );

    let entry = documents.get(&source).unwrap();
    let Model::Java(file) = &entry.model else {
        panic!("expected a Java model");
    };
    let Content::Stored(bytes) = entry.content.as_ref().unwrap();
    assert_eq!(file.package_name.as_slice(), ["example"]);
    assert_eq!(bytes.as_slice(), contents.as_bytes());
    assert!(documents.get(&ResourceId::file("/Missing.java")).is_none());
}

#[test]
fn document_entries_can_be_replaced_and_removed_without_retained_content() {
    let mut documents = Documents::new();
    let source = ResourceId::file("/Example.class");
    let entry = |name: &str| Entry {
        model: Model::Jvm(beans_platform_jvm::Class { name: name.into() }),
        content: None,
    };

    assert!(documents.insert(source.clone(), entry("First")).is_none());
    let previous = documents.insert(source.clone(), entry("Second")).unwrap();
    let Model::Jvm(previous) = previous.model else {
        panic!("expected a JVM model");
    };
    assert_eq!(previous.name, "First");

    let current = documents.get(&source).unwrap();
    let Model::Jvm(current) = &current.model else {
        panic!("expected a JVM model");
    };
    assert_eq!(current.name, "Second");

    let removed = documents.remove(&source).unwrap();
    assert!(removed.content.is_none());
    let Model::Jvm(removed) = removed.model else {
        panic!("expected a JVM model");
    };
    assert_eq!(removed.name, "Second");
    assert!(documents.get(&source).is_none());
    assert!(documents.remove(&source).is_none());
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
                .java
                .file(engine.revision(), &ResourceId::file("/Example.java"))
                .is_none()
        );
    }

    engine.document_close(source.clone());
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
    engine.process(ResourceId::file("/lib/Example.class"), "not Java source");
    engine.process(virtual_source.clone(), "class C extends T {}");
    engine.process(
        ResourceId::file("/sources.jar").entry(ResourceEntry("Example.java".into())),
        "class C extends T {}",
    );
    engine.document_open(virtual_source.clone(), "plaintext", "class C extends T {}");
    engine.document_edit(virtual_source.clone(), "plaintext", "class C extends T {}");

    assert_eq!(engine.revision(), Revision::default());
    assert!(
        engine
            .java
            .file(engine.revision(), &virtual_source)
            .is_none()
    );
}
