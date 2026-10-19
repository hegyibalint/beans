use beans_core::{
    classpath::Unrestricted, location::Location, names::Name, query_scope::QueryScope,
    resolution::TypeDefinitionQuery,
};
use beans_engine::storage::open;
use beans_lang_java::{
    lower_into,
    semantics::resolution::{TypeCandidate, query::ResolutionQuery},
    storage as java_storage,
};
use beans_platform_jvm::{Class, storage as jvm_storage};
use rusqlite::Connection;

#[test]
fn java_and_jvm_queries_share_a_snapshot_across_replacement_and_deletion() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    let mut writer = open(&path).unwrap();
    publish(&mut writer, "Old");
    let mut reader = open(&path).unwrap();
    let old = Name::new(vec!["p".into(), "Old".into()]);
    let new = Name::new(vec!["p".into(), "New".into()]);

    {
        let transaction = reader.transaction().unwrap();
        let query = ResolutionQuery::new(&transaction, QueryScope::new(&Unrestricted));
        let candidates = query.find_types(&old).unwrap();
        assert!(matches!(
            candidates.as_slice(),
            [TypeCandidate::Java(_), TypeCandidate::Jvm(_)]
        ));

        publish(&mut writer, "New");

        assert_eq!(query.find_types(&old).unwrap(), candidates);
        assert!(query.find_types(&new).unwrap().is_empty());
    }
    {
        let transaction = reader.transaction().unwrap();
        let query = ResolutionQuery::new(&transaction, QueryScope::new(&Unrestricted));
        assert!(query.find_types(&old).unwrap().is_empty());
        let candidates = query.find_types(&new).unwrap();
        assert!(matches!(
            candidates.as_slice(),
            [TypeCandidate::Java(_), TypeCandidate::Jvm(_)]
        ));

        let publication = writer.transaction().unwrap();
        java_storage::remove(&publication, &Location::file("/Example.java")).unwrap();
        jvm_storage::remove(&publication, "/Example.class".as_ref()).unwrap();
        publication.commit().unwrap();

        assert_eq!(query.find_types(&new).unwrap(), candidates);
    }
    let query = ResolutionQuery::new(&reader, QueryScope::new(&Unrestricted));
    assert!(query.find_types(&old).unwrap().is_empty());
    assert!(query.find_types(&new).unwrap().is_empty());
}

fn publish(connection: &mut Connection, name: &str) {
    let transaction = connection.transaction().unwrap();
    let contents = format!("package p; class {name} {{}}");
    java_storage::store(
        &transaction,
        &lower_into(&contents),
        &Location::file("/Example.java"),
        &contents,
    )
    .unwrap();
    jvm_storage::remove(&transaction, "/Example.class".as_ref()).unwrap();
    jvm_storage::store(
        &transaction,
        &Class {
            name: format!("p/{name}"),
        },
        &Location::file("/Example.class"),
    )
    .unwrap();
    transaction.commit().unwrap();
}
