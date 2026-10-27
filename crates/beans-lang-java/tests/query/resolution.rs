use beans_core::{location::Location, names::Name};
use beans_lang_java::{
    query as java_query,
    semantics::resolution::{JavaTypeCandidate, TypeCandidate, query},
};
use beans_testing::database;

use super::support::Models;

#[test]
fn native_java_addresses_are_wrapped_as_candidates() {
    let name = Name::new(vec!["example".into(), "Example".into()]);
    let mut models = Models::new();
    models.store(
        models.process("package example; class Example {}"),
        Location::file("/src/Example.java"),
    );
    let address = java_query::find_type(&name, &models.connection)
        .unwrap()
        .remove(0);
    assert_eq!(
        query::find_types(&name, &models.connection).unwrap(),
        [TypeCandidate::Java(JavaTypeCandidate::Declaration(address))]
    );
}

#[test]
fn jvm_storage_errors_are_not_missing_candidates() {
    let connection = database();
    connection.execute("DROP TABLE models", []).unwrap();
    assert!(query::find_types(&Name::new(vec!["Missing".into()]), &connection).is_err());
}

#[test]
fn java_storage_errors_are_not_missing_candidates() {
    let connection = database();
    connection.execute("DROP TABLE java_types", []).unwrap();
    assert!(query::find_types(&Name::new(vec!["Missing".into()]), &connection).is_err());
}
