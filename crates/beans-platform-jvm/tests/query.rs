use beans_core::{
    classpath::Classpath,
    location::{Location, LocationEntry},
    query_scope::QueryScope,
};
use beans_testing::database;
use rusqlite::Connection;

use beans_platform_jvm::{Class, query::JvmQuery, storage};

#[test]
fn finds_all_jvm_locations_by_exact_internal_name() {
    let mut connection = database();
    let first = Location::file("/Foo.class");
    let second =
        Location::file("/library.jar").entry(LocationEntry::Container("p/Foo.class".into()));
    store(&mut connection, "p/Foo", &first);
    store(&mut connection, "p/Foo", &second);
    store(
        &mut connection,
        "p/FooBar",
        &Location::file("/FooBar.class"),
    );
    connection
        .execute(
            r#"INSERT INTO models (location, kind, model, content)
               VALUES ('[]', 1, '{"name":"p/Foo"}', X'')"#,
            [],
        )
        .unwrap();
    let query = JvmQuery::new(&connection, QueryScope::unrestricted());

    assert_eq!(query.find_classes("p/Foo").unwrap(), [first, second]);
    for name in ["Missing", "p.Foo", "p/foo", "p/F"] {
        assert!(query.find_classes(name).unwrap().is_empty());
    }
}

#[test]
fn classpath_membership_uses_classes_not_sources() {
    let mut connection = database();
    let visible =
        Location::file("/visible.jar").entry(LocationEntry::Container("p/Foo.class".into()));
    store(
        &mut connection,
        "p/Foo",
        &Location::file("/hidden/Foo.class"),
    );
    store(&mut connection, "p/Foo", &visible);
    let sources = [Classpath::Unrestricted];
    let classes = [Classpath::file("/visible.jar")];
    let query = JvmQuery::new(&connection, QueryScope::new(&sources, &classes));

    assert_eq!(query.find_classes("p/Foo").unwrap(), [visible]);
}

#[test]
fn database_errors_are_not_missing_classes() {
    let connection = database();
    connection.execute("DROP TABLE models", []).unwrap();
    let query = JvmQuery::new(&connection, QueryScope::unrestricted());

    assert!(query.find_classes("p/Foo").is_err());
}

#[test]
fn invalid_matching_locations_are_errors_but_unrelated_rows_are_not_decoded() {
    let mut connection = database();
    let location = Location::file("/Foo.class");
    store(&mut connection, "p/Foo", &location);
    connection
        .execute(
            r#"INSERT INTO models (location, kind, model)
               VALUES ('{}', 0, '{"name":"p/Broken"}')"#,
            [],
        )
        .unwrap();
    let query = JvmQuery::new(&connection, QueryScope::unrestricted());

    assert_eq!(query.find_classes("p/Foo").unwrap(), [location]);
    assert!(query.find_classes("p/Missing").unwrap().is_empty());
    assert!(query.find_classes("p/Broken").is_err());
}

#[test]
fn queries_borrow_the_callers_transaction_and_follow_index_replacement_and_deletion() {
    let mut connection = database();
    let location = Location::file("/Foo.class");
    store(&mut connection, "p/Old", &location);
    {
        let transaction = connection.transaction().unwrap();
        storage::remove(&transaction, "/Foo.class".as_ref()).unwrap();
        storage::store(
            &transaction,
            &Class {
                name: "p/New".into(),
            },
            &location,
        )
        .unwrap();
        let query = JvmQuery::new(&transaction, QueryScope::unrestricted());

        assert!(query.find_classes("p/Old").unwrap().is_empty());
        assert_eq!(
            query.find_classes("p/New").unwrap(),
            std::slice::from_ref(&location)
        );
    }
    let query = JvmQuery::new(&connection, QueryScope::unrestricted());
    assert_eq!(
        query.find_classes("p/Old").unwrap(),
        std::slice::from_ref(&location)
    );
    assert!(query.find_classes("p/New").unwrap().is_empty());

    let transaction = connection.transaction().unwrap();
    storage::remove(&transaction, "/Foo.class".as_ref()).unwrap();
    storage::store(
        &transaction,
        &Class {
            name: "p/New".into(),
        },
        &location,
    )
    .unwrap();
    transaction.commit().unwrap();
    let query = JvmQuery::new(&connection, QueryScope::unrestricted());
    assert!(query.find_classes("p/Old").unwrap().is_empty());
    assert_eq!(query.find_classes("p/New").unwrap(), [location]);

    let transaction = connection.transaction().unwrap();
    storage::remove(&transaction, "/Foo.class".as_ref()).unwrap();
    transaction.commit().unwrap();
    let query = JvmQuery::new(&connection, QueryScope::unrestricted());
    assert!(query.find_classes("p/New").unwrap().is_empty());
}

fn store(connection: &mut Connection, name: &str, location: &Location) {
    let transaction = connection.transaction().unwrap();
    storage::store(&transaction, &Class { name: name.into() }, location).unwrap();
    transaction.commit().unwrap();
}
