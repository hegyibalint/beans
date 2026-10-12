use beans_core::resource::ResourceId;

use super::{Content, Entry, Model, Storage};

#[test]
fn entries_keep_the_model_and_retained_input_together() {
    let mut storage = Storage::new();
    let source = ResourceId::file("/src/example/Example.java");
    let contents = "package example; class Example {}";
    let model = beans_lang_java::process(&source, contents).unwrap();

    assert!(
        storage
            .insert(
                source.clone(),
                Entry {
                    model: Model::Java(model),
                    content: Some(Content::Stored(contents.as_bytes().to_vec())),
                },
            )
            .is_none()
    );

    let entry = storage.get(&source).unwrap();
    let Model::Java(file) = &entry.model else {
        panic!("expected a Java model");
    };
    let Content::Stored(bytes) = entry.content.as_ref().unwrap();
    assert_eq!(file.package_name.as_slice(), ["example"]);
    assert_eq!(bytes.as_slice(), contents.as_bytes());
    assert!(storage.get(&ResourceId::file("/Missing.java")).is_none());
}

#[test]
fn entries_can_be_replaced_and_removed_without_retained_content() {
    let mut storage = Storage::new();
    let source = ResourceId::file("/Example.class");
    let entry = |name: &str| Entry {
        model: Model::Jvm(beans_platform_jvm::Class { name: name.into() }),
        content: None,
    };

    assert!(storage.insert(source.clone(), entry("First")).is_none());
    let previous = storage.insert(source.clone(), entry("Second")).unwrap();
    let Model::Jvm(previous) = previous.model else {
        panic!("expected a JVM model");
    };
    assert_eq!(previous.name, "First");

    let current = storage.get(&source).unwrap();
    let Model::Jvm(current) = &current.model else {
        panic!("expected a JVM model");
    };
    assert_eq!(current.name, "Second");

    let removed = storage.remove(&source).unwrap();
    assert!(removed.content.is_none());
    let Model::Jvm(removed) = removed.model else {
        panic!("expected a JVM model");
    };
    assert_eq!(removed.name, "Second");
    assert!(storage.get(&source).is_none());
    assert!(storage.remove(&source).is_none());
}
