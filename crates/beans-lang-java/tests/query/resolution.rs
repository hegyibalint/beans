use beans_core::{
    classpath::Unrestricted, location::Location, names::Name, query_scope::QueryScope,
    resolution::TypeDefinitionQuery,
};
use beans_lang_java::semantics::resolution::{
    JavaTypeCandidate, TypeCandidate, query::ResolutionQuery,
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
    let java = models.query(QueryScope::new(&Unrestricted));
    let address = java.find_types(&name).unwrap().remove(0);
    let query = ResolutionQuery::new(&models.connection, QueryScope::new(&Unrestricted));
    assert_eq!(
        query.find_types(&name).unwrap(),
        [TypeCandidate::Java(JavaTypeCandidate::Declaration(address))]
    );
}

#[test]
fn jvm_storage_errors_are_not_missing_candidates() {
    let connection = database();
    connection.execute("DROP TABLE models", []).unwrap();
    let query = ResolutionQuery::new(&connection, QueryScope::new(&Unrestricted));
    assert!(
        query
            .find_types(&Name::new(vec!["Missing".into()]))
            .is_err()
    );
}

#[test]
fn java_storage_errors_are_not_missing_candidates() {
    let connection = database();
    connection.execute("DROP TABLE java_types", []).unwrap();
    let query = ResolutionQuery::new(&connection, QueryScope::new(&Unrestricted));
    assert!(
        query
            .find_types(&Name::new(vec!["Missing".into()]))
            .is_err()
    );
}
