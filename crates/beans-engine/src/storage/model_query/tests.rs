use beans_core::{
    names::Name,
    resource::{ResourceEntry, ResourceRoot},
};
use rusqlite::{OpenFlags, params};
use tempfile::tempdir;

use super::*;
use crate::storage;

#[test]
fn java_queries_return_owned_models_and_matching_content_after_reopening() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let source = ResourceId::new(ResourceRoot::Virtual {
        provider: "lsp".into(),
        key: "untitled:Untitled-1".into(),
    });
    let content = "package p; class Example<T> { T field; class Inner {} }";
    {
        let writer = storage::open(&path).unwrap();
        put_java(&writer, &source, 1, content);
    }
    let (file, bytes) = {
        let mut reader = reader(&path);
        let query = ModelQuery::new(&mut reader).unwrap();
        assert_eq!(query.revision(), Revision::new(1));
        (
            query.java_file(&source).unwrap().unwrap(),
            query.content(&source).unwrap().unwrap(),
        )
    };
    assert_eq!(bytes, content.as_bytes());
    assert_eq!(file.package_name.as_slice(), ["p"]);
    assert_eq!(
        file.find_type(&Name::new(vec![
            "p".into(),
            "Example".into(),
            "Inner".into()
        ]))[0]
            .1
            .name(),
        Some("Inner")
    );
}

#[test]
fn jvm_queries_keep_full_nested_identity_and_return_owned_classes() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let container = ResourceId::file("app.zip");
    let source = container
        .entry(ResourceEntry("lib/a.jar".into()))
        .entry(ResourceEntry("p/C.class".into()));
    put_jvm(&writer, &source, 1, "p/C");
    let class = {
        let mut reader = reader(&path);
        let query = ModelQuery::new(&mut reader).unwrap();
        assert!(query.jvm_class(&container).unwrap().is_none());
        assert!(query.java_file(&source).unwrap().is_none());
        assert!(query.content(&source).unwrap().is_none());
        query.jvm_class(&source).unwrap().unwrap()
    };
    assert_eq!(class.name, "p/C");
}

#[test]
fn empty_snapshots_and_missing_resources_return_none() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let _writer = storage::open(&path).unwrap();
    let mut reader = reader(&path);
    let query = ModelQuery::new(&mut reader).unwrap();
    let source = ResourceId::file("Missing");
    assert_eq!(query.revision(), Revision::new(0));
    assert!(query.java_file(&source).unwrap().is_none());
    assert!(query.jvm_class(&source).unwrap().is_none());
    assert!(query.content(&source).unwrap().is_none());
}

#[test]
fn a_query_keeps_its_revision_and_models_when_the_writer_publishes_more_rows() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Example");
    let future = ResourceId::file("Future");
    put_java(&writer, &source, 1, "package old; class Example {}");
    let mut reader = reader(&path);
    {
        let query = ModelQuery::new(&mut reader).unwrap();
        assert_eq!(query.revision(), Revision::new(1));
        put_jvm(&writer, &source, 2, "new/Example");
        put_java(&writer, &future, 3, "class Future {}");
        assert_eq!(
            query
                .java_file(&source)
                .unwrap()
                .unwrap()
                .package_name
                .as_slice(),
            ["old"]
        );
        assert!(query.jvm_class(&source).unwrap().is_none());
        assert!(query.java_file(&future).unwrap().is_none());
        assert_eq!(
            query.content(&source).unwrap().unwrap(),
            b"package old; class Example {}"
        );
    }
    let query = ModelQuery::new(&mut reader).unwrap();
    assert_eq!(query.revision(), Revision::new(3));
    assert!(query.java_file(&source).unwrap().is_none());
    assert_eq!(
        query.jvm_class(&source).unwrap().unwrap().name,
        "new/Example"
    );
    assert!(query.java_file(&future).unwrap().is_some());
}

#[test]
fn historical_queries_select_versions_before_filtering_by_kind() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Example");
    put_jvm(&writer, &source, 1, "old/Example");
    put_java(&writer, &source, 2, "class Example {}");
    let mut reader = reader(&path);
    {
        let query = ModelQuery::at_revision(&mut reader, Revision::new(1)).unwrap();
        assert_eq!(
            query.jvm_class(&source).unwrap().unwrap().name,
            "old/Example"
        );
        assert!(query.java_file(&source).unwrap().is_none());
    }
    let query = ModelQuery::at_revision(&mut reader, Revision::new(2)).unwrap();
    assert!(query.jvm_class(&source).unwrap().is_none());
    assert!(query.java_file(&source).unwrap().is_some());
}

#[test]
fn each_retrieval_gives_the_caller_an_independent_model() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Example");
    put_java(&writer, &source, 1, "package p; class Example {}");
    let mut reader = reader(&path);
    let query = ModelQuery::new(&mut reader).unwrap();
    let mut first = query.java_file(&source).unwrap().unwrap();
    first.package_name = Name::new(vec!["changed".into()]);
    let second = query.java_file(&source).unwrap().unwrap();
    assert_eq!(second.package_name.as_slice(), ["p"]);
    assert!(!std::ptr::eq(
        first.node(File::ROOT_NODE_ID).unwrap(),
        second.node(File::ROOT_NODE_ID).unwrap()
    ));
}

#[test]
fn invalid_model_versions_and_payloads_are_errors_not_missing_models() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Example");
    let mut reader = reader(&path);
    for (revision, kind, bytes, content) in [
        (1, 0, vec![2], None),
        (2, 0, vec![1, 255], None),
        (3, 1, vec![2], Some(b"source".as_slice())),
        (4, 1, vec![1, 255], Some(b"source".as_slice())),
    ] {
        insert(&writer, &source, revision, kind, &bytes, content);
        let query = ModelQuery::new(&mut reader).unwrap();
        if kind == 0 {
            assert!(query.jvm_class(&source).is_err());
            assert!(query.java_file(&source).unwrap().is_none());
        } else {
            assert!(query.java_file(&source).is_err());
            assert!(query.jvm_class(&source).unwrap().is_none());
        }
    }
}

#[test]
fn queries_reject_future_revisions() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Example");
    put_jvm(&writer, &source, 1, "p/Example");
    let mut reader = reader(&path);
    assert!(ModelQuery::at_revision(&mut reader, Revision::new(2)).is_err());
    let query = ModelQuery::at_revision(&mut reader, Revision::new(0)).unwrap();
    assert!(query.jvm_class(&source).unwrap().is_none());
}

#[test]
fn queries_support_the_maximum_revision() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Example");
    put_jvm(&writer, &source, i64::MAX, "p/Example");
    let mut reader = reader(&path);
    let query = ModelQuery::at_revision(&mut reader, Revision::new(i64::MAX)).unwrap();
    assert_eq!(query.revision(), Revision::new(i64::MAX));
    assert_eq!(query.jvm_class(&source).unwrap().unwrap().name, "p/Example");
}

#[test]
fn negative_database_revisions_return_an_error_without_panicking() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    writer
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    put_jvm(&writer, &ResourceId::file("Example"), -1, "p/Example");
    let mut reader = reader(&path);
    assert_eq!(
        ModelQuery::new(&mut reader).err().unwrap(),
        "negative database revision"
    );
}

#[test]
fn unknown_kind_tags_are_errors_even_if_constraints_were_bypassed() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    writer
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    let source = ResourceId::file("Example");
    insert(&writer, &source, 1, 7, &[1], None);
    let mut reader = reader(&path);
    let query = ModelQuery::new(&mut reader).unwrap();
    assert_eq!(
        query.java_file(&source).unwrap_err(),
        "unknown model kind: 7"
    );
    assert_eq!(
        query.jvm_class(&source).err().unwrap(),
        "unknown model kind: 7"
    );
    assert_eq!(query.java_files().unwrap_err(), "unknown model kind: 7");
    assert_eq!(query.jvm_classes().err().unwrap(), "unknown model kind: 7");
}

#[test]
fn sql_errors_are_not_reported_as_missing_models() {
    let root = tempdir().unwrap();
    let mut connection = Connection::open(root.path().join("uninitialized.sqlite")).unwrap();
    assert!(ModelQuery::new(&mut connection).is_err());
}

#[test]
fn trait_enumeration_returns_owned_models_without_resurrecting_replaced_kinds() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Changed");
    let nested = ResourceId::file("app.jar").entry(ResourceEntry("p/C.class".into()));
    let virtual_source = ResourceId::new(ResourceRoot::Virtual {
        provider: "lsp".into(),
        key: "untitled:Untitled-1".into(),
    });
    put_jvm(&writer, &source, 1, "old/Changed");
    put_jvm(&writer, &nested, 1, "p/C");
    put_java(&writer, &source, 2, "package p; class Changed {}");
    put_java(&writer, &virtual_source, 2, "class Unsaved {}");
    let mut reader = reader(&path);
    {
        let query = ModelQuery::at_revision(&mut reader, Revision::new(1)).unwrap();
        assert!(JavaModelQuery::java_files(&query).unwrap().is_empty());
        assert_eq!(JvmModelQuery::jvm_classes(&query).unwrap().len(), 2);
    }
    let (files, classes) = {
        let query = ModelQuery::new(&mut reader).unwrap();
        (
            JavaModelQuery::java_files(&query).unwrap(),
            JvmModelQuery::jvm_classes(&query).unwrap(),
        )
    };
    assert_eq!(files.len(), 2);
    assert!(
        files
            .iter()
            .any(|(id, file)| id == &source && file.package_name.as_slice() == ["p"])
    );
    assert!(files.iter().any(|(id, _)| id == &virtual_source));
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0].0, nested);
    assert_eq!(classes[0].1.name, "p/C");
}

#[test]
fn trait_enumeration_uses_the_pinned_snapshot() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Example");
    put_java(&writer, &source, 1, "package old; class Example {}");
    let mut reader = reader(&path);
    let query = ModelQuery::new(&mut reader).unwrap();
    put_jvm(&writer, &source, 2, "new/Example");
    assert_eq!(
        query.java_files().unwrap()[0].1.package_name.as_slice(),
        ["old"]
    );
    assert!(query.jvm_classes().unwrap().is_empty());
}

#[test]
fn enumeration_propagates_decode_and_resource_url_errors() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Example");
    insert(&writer, &source, 1, 0, &[2], None);
    let mut reader = reader(&path);
    {
        let query = ModelQuery::new(&mut reader).unwrap();
        assert!(query.java_files().unwrap().is_empty());
        assert_eq!(
            query.jvm_classes().err().unwrap(),
            "unsupported JVM model version: 2"
        );
    }
    put_java(&writer, &source, 2, "class Example {}");
    let bytes = beans_platform_jvm::codec::encode(&Class { name: "p/C".into() }).unwrap();
    writer
        .execute(
            "INSERT INTO entries VALUES ('not-a-resource-url', 2, 0, ?1, NULL)",
            [bytes],
        )
        .unwrap();
    let query = ModelQuery::new(&mut reader).unwrap();
    assert_eq!(query.java_files().unwrap().len(), 1);
    assert!(query.jvm_classes().is_err());
}

#[test]
fn java_features_use_the_sql_traits_and_propagate_external_query_failures() {
    use beans_core::{
        classpath::Unrestricted,
        features::{DefinitionRequest, HoverRequest},
        query_scope::QueryScope,
    };
    use beans_lang_java::features::{JavaFeatureContext, goto_definition, hover};
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Use.java");
    let target = ResourceId::file("Target.java");
    let text = "package p; import q.Target; class Use { Target field; }";
    let target_text = "package q; class Target {}";
    put_java(&writer, &source, 1, text);
    put_java(&writer, &target, 1, target_text);
    let mut reader = reader(&path);
    {
        let query = ModelQuery::new(&mut reader).unwrap();
        let context = JavaFeatureContext {
            models: &query,
            scope: QueryScope::new(query.revision(), &Unrestricted),
        };
        let request = DefinitionRequest {
            source: &source,
            offset: text.rfind("Target").unwrap(),
        };
        assert_eq!(
            goto_definition(&context, &request)
                .unwrap()
                .unwrap()
                .target
                .resource,
            target
        );
        let hover = hover(
            &context,
            &HoverRequest {
                source: &source,
                offset: request.offset,
                contents: text,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(hover.contents(), "Target");
    }
    insert(&writer, &target, 2, 1, &[2], Some(target_text.as_bytes()));
    let query = ModelQuery::new(&mut reader).unwrap();
    let context = JavaFeatureContext {
        models: &query,
        scope: QueryScope::new(query.revision(), &Unrestricted),
    };
    let request = DefinitionRequest {
        source: &source,
        offset: text.rfind("Target").unwrap(),
    };
    assert_eq!(
        goto_definition(&context, &request).err().unwrap(),
        "unsupported Java model version: 2"
    );
    assert_eq!(
        hover(
            &context,
            &HoverRequest {
                source: &target,
                contents: target_text,
                offset: 0
            }
        )
        .err()
        .unwrap(),
        "unsupported Java model version: 2"
    );
}

#[test]
fn inherited_lookup_does_not_hide_query_failures_behind_a_local_candidate() {
    use beans_core::{
        classpath::Unrestricted, features::DefinitionRequest, query_scope::QueryScope,
    };
    use beans_lang_java::features::{JavaFeatureContext, goto_definition};
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let writer = storage::open(&path).unwrap();
    let source = ResourceId::file("Use.java");
    let text = "class Target {} class Use extends q.Missing { Target field; }";
    put_java(&writer, &source, 1, text);
    insert(
        &writer,
        &ResourceId::file("Corrupt.java"),
        1,
        1,
        &[2],
        Some(b"source"),
    );
    let mut reader = reader(&path);
    let query = ModelQuery::new(&mut reader).unwrap();
    let context = JavaFeatureContext {
        models: &query,
        scope: QueryScope::new(query.revision(), &Unrestricted),
    };
    assert_eq!(
        goto_definition(
            &context,
            &DefinitionRequest {
                source: &source,
                offset: text.rfind("Target").unwrap()
            }
        )
        .err()
        .unwrap(),
        "unsupported Java model version: 2"
    );
}

fn reader(path: &std::path::Path) -> Connection {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
}

fn put_java(connection: &Connection, source: &ResourceId, revision: i64, content: &str) {
    let file = beans_lang_java::process(source, content).unwrap();
    let bytes = beans_lang_java::codec::encode(&file).unwrap();
    insert(
        connection,
        source,
        revision,
        1,
        &bytes,
        Some(content.as_bytes()),
    );
}

fn put_jvm(connection: &Connection, source: &ResourceId, revision: i64, name: &str) {
    let bytes = beans_platform_jvm::codec::encode(&Class { name: name.into() }).unwrap();
    insert(connection, source, revision, 0, &bytes, None);
}

fn insert(
    connection: &Connection,
    source: &ResourceId,
    revision: i64,
    kind: i64,
    bytes: &[u8],
    content: Option<&[u8]>,
) {
    connection
        .execute(
            "INSERT INTO entries VALUES (?1, ?2, ?3, ?4, ?5)",
            params![source.to_url().unwrap(), revision, kind, bytes, content],
        )
        .unwrap();
}
