use beans_core::{location::Location, names::Name};
use beans_engine::storage::open;
use beans_lang_java::{
    lower_into,
    semantics::resolution::{TypeCandidate, query},
    storage as java_storage,
};
use beans_platform_jvm::{Class, storage as jvm_storage};
use beans_testing::TempDir;
use rusqlite::Connection;

#[test]
fn java_and_jvm_queries_share_a_snapshot_across_replacement_and_deletion() {
    let root = TempDir::new();
    let path = root.path("models.sqlite");
    let mut writer = open(&path).unwrap();
    publish("Old", &mut writer);
    let mut reader = open(&path).unwrap();
    let old = Name::new(vec!["p".into(), "Old".into()]);
    let new = Name::new(vec!["p".into(), "New".into()]);

    {
        let transaction = reader.transaction().unwrap();
        let candidates = query::find_types(&old, &transaction).unwrap();
        assert!(matches!(
            candidates.as_slice(),
            [TypeCandidate::Java(_), TypeCandidate::Jvm(_)]
        ));

        publish("New", &mut writer);

        assert_eq!(query::find_types(&old, &transaction).unwrap(), candidates);
        assert!(query::find_types(&new, &transaction).unwrap().is_empty());
    }
    {
        let transaction = reader.transaction().unwrap();
        assert!(query::find_types(&old, &transaction).unwrap().is_empty());
        let candidates = query::find_types(&new, &transaction).unwrap();
        assert!(matches!(
            candidates.as_slice(),
            [TypeCandidate::Java(_), TypeCandidate::Jvm(_)]
        ));

        let publication = writer.transaction().unwrap();
        java_storage::remove(&Location::file("/Example.java"), &publication).unwrap();
        jvm_storage::remove("/Example.class".as_ref(), &publication).unwrap();
        publication.commit().unwrap();

        assert_eq!(query::find_types(&new, &transaction).unwrap(), candidates);
    }
    assert!(query::find_types(&old, &reader).unwrap().is_empty());
    assert!(query::find_types(&new, &reader).unwrap().is_empty());
}

fn publish(name: &str, connection: &mut Connection) {
    let transaction = connection.transaction().unwrap();
    let contents = format!("package p; class {name} {{}}");
    java_storage::store(
        &lower_into(&contents),
        &Location::file("/Example.java"),
        &contents,
        &transaction,
    )
    .unwrap();
    jvm_storage::remove("/Example.class".as_ref(), &transaction).unwrap();
    jvm_storage::store(
        &Class {
            name: format!("p/{name}"),
        },
        &Location::file("/Example.class"),
        &transaction,
    )
    .unwrap();
    transaction.commit().unwrap();
}
