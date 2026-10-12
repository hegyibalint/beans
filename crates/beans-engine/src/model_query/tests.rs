use beans_core::resource::{ResourceEntry, ResourceId};
use beans_lang_java::{JavaLanguage, model_query::JavaModelQuery};
use beans_platform_jvm::{Class, model_query::JvmModelQuery};

use super::ModelQuery;
use crate::storage::{Entry, Model, Storage};

#[test]
fn java_model_reads_borrow_the_stored_entry() {
    let mut storage = Storage::new();
    let source = ResourceId::file("/Example.java");
    storage.insert(
        source.clone(),
        Entry {
            model: Model::Java(
                JavaLanguage::new()
                    .process(&source, "package example; class Example {}")
                    .unwrap(),
            ),
            content: None,
        },
    );
    let query = ModelQuery::new(&storage);

    let file = query.java_file(&source).unwrap();
    assert_eq!(file.package_name.as_slice(), ["example"]);
    let Model::Java(stored) = &storage.get(&source).unwrap().model else {
        panic!("expected a Java model");
    };
    assert!(std::ptr::eq(file, stored));
    assert!(query.jvm_class(&source).is_none());
}

#[test]
fn jvm_model_reads_borrow_the_entry_at_the_full_resource_id() {
    let mut storage = Storage::new();
    let container = ResourceId::file("/library.jar");
    let source = container
        .clone()
        .entry(ResourceEntry("example/Example.class".into()));
    storage.insert(
        source.clone(),
        Entry {
            model: Model::Jvm(Class {
                name: "example/Example".into(),
            }),
            content: None,
        },
    );
    let query = ModelQuery::new(&storage);

    let class = query.jvm_class(&source).unwrap();
    assert_eq!(class.name, "example/Example");
    let Model::Jvm(stored) = &storage.get(&source).unwrap().model else {
        panic!("expected a JVM model");
    };
    assert!(std::ptr::eq(class, stored));
    assert!(query.java_file(&source).is_none());
    assert!(query.jvm_class(&container).is_none());
}

#[test]
fn enumeration_returns_only_borrowed_java_models() {
    let mut storage = Storage::new();
    let java_source = ResourceId::file("/Example.java");
    storage.insert(
        java_source.clone(),
        Entry {
            model: Model::Java(
                JavaLanguage::new()
                    .process(&java_source, "class Example {}")
                    .unwrap(),
            ),
            content: None,
        },
    );
    storage.insert(
        ResourceId::file("/Example.class"),
        Entry {
            model: Model::Jvm(Class {
                name: "Example".into(),
            }),
            content: None,
        },
    );
    let query = ModelQuery::new(&storage);
    let files: Vec<_> = query.java_files().collect();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, &java_source);
    assert!(std::ptr::eq(
        files[0].1,
        query.java_file(&java_source).unwrap()
    ));
}

#[test]
fn unknown_resources_have_no_java_or_jvm_model() {
    let storage = Storage::new();
    let query = ModelQuery::new(&storage);
    let source = ResourceId::file("/Missing.java");

    assert!(query.java_file(&source).is_none());
    assert!(query.jvm_class(&source).is_none());
}
