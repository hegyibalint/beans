use beans_core::resource::{ResourceEntry, ResourceId};
use beans_lang_java::model_query::JavaModelQuery;
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
                beans_lang_java::process(&source, "package example; class Example {}").unwrap(),
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
    let classes: Vec<_> = query.jvm_classes().collect();
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0].0, &source);
    assert!(std::ptr::eq(classes[0].1, class));
}

#[test]
fn enumeration_returns_only_borrowed_java_models() {
    let mut storage = Storage::new();
    let java_source = ResourceId::file("/Example.java");
    storage.insert(
        java_source.clone(),
        Entry {
            model: Model::Java(beans_lang_java::process(&java_source, "class Example {}").unwrap()),
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
fn jvm_discovery_uses_replaced_and_removed_storage_entries() {
    use beans_core::{classpath::Unrestricted, query_scope::QueryScope, revision::Revision};

    let mut storage = Storage::new();
    let source = ResourceId::file("/Example.class");
    let scope = QueryScope::new(Revision::new(7), &Unrestricted);
    for name in ["example/Former", "example/Latest"] {
        storage.insert(
            source.clone(),
            Entry {
                model: Model::Jvm(Class { name: name.into() }),
                content: None,
            },
        );
        let query = ModelQuery::new(&storage);
        let found = beans_platform_jvm::find_classes(&query, scope, name);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, source);
        assert_eq!(found[0].revision, scope.revision());
        if name == "example/Latest" {
            assert!(beans_platform_jvm::find_classes(&query, scope, "example/Former").is_empty());
        }
    }
    storage.remove(&source);
    let query = ModelQuery::new(&storage);
    assert!(query.jvm_class(&source).is_none());
    assert!(query.jvm_classes().next().is_none());
    assert!(beans_platform_jvm::find_classes(&query, scope, "example/Latest").is_empty());
}

#[test]
fn java_resolution_consumes_jvm_models_from_the_same_engine_storage() {
    use beans_core::{classpath::Unrestricted, query_scope::QueryScope, revision::Revision};
    use beans_lang_java::{
        model::nodes::NodeKind,
        query::JavaQuery,
        semantics::resolution::{Context, TypeCandidate, query::ResolutionQuery, resolve},
    };

    let mut storage = Storage::new();
    let source = ResourceId::file("/Use.java");
    let class_source =
        ResourceId::file("/runtime.jar").entry(ResourceEntry("java/lang/String.class".into()));
    storage.insert(
        source.clone(),
        Entry {
            model: Model::Java(
                beans_lang_java::process(&source, "class Use { String field; }").unwrap(),
            ),
            content: None,
        },
    );
    storage.insert(
        class_source.clone(),
        Entry {
            model: Model::Jvm(Class {
                name: "java/lang/String".into(),
            }),
            content: None,
        },
    );
    let models = ModelQuery::new(&storage);
    let revision = Revision::new(7);
    let java = JavaQuery::new(&models, QueryScope::new(revision, &Unrestricted));
    let definitions = ResolutionQuery::new(&java, &models);
    let file = java.file(&source).unwrap();
    let field = file
        .iter_nodes()
        .find(|entry| matches!(entry.node.kind(), NodeKind::Field(_)))
        .unwrap();
    let NodeKind::Field(declaration) = field.node.kind() else {
        unreachable!()
    };
    let context = Context::new(
        revision,
        &source,
        file,
        field.index,
        declaration.declared_type.value(),
    )
    .with_definitions(&definitions);

    let TypeCandidate::Jvm(address) = resolve(&context).unwrap() else {
        panic!("expected an Engine-stored JVM candidate");
    };
    assert_eq!(address.key, class_source);
    assert_eq!(address.revision, revision);
}

#[test]
fn unknown_resources_have_no_java_or_jvm_model() {
    let storage = Storage::new();
    let query = ModelQuery::new(&storage);
    let source = ResourceId::file("/Missing.java");

    assert!(query.java_file(&source).is_none());
    assert!(query.jvm_class(&source).is_none());
}
