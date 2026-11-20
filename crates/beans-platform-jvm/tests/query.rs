use beans_core::location::{Location, LocationEntry};
use beans_testing::{database, support::class_model};
use rusqlite::Connection;

use beans_platform_jvm::{query, storage};

#[test]
fn finds_all_jvm_locations_by_exact_internal_name() {
    let mut connection = database();
    let first = Location::file("/Foo.class");
    let second =
        Location::file("/library.jar").entry(LocationEntry::Container("p/Foo.class".into()));
    store("p/Foo", &first, &mut connection);
    store("p/Foo", &second, &mut connection);
    store(
        "p/FooBar",
        &Location::file("/FooBar.class"),
        &mut connection,
    );
    connection
        .execute(
            r#"INSERT INTO models (location, kind, model, content)
               VALUES ('[]', 1, '{"name":"p/Foo"}', X'')"#,
            [],
        )
        .unwrap();
    assert_eq!(
        query::find_classes("p/Foo", &connection).unwrap(),
        [first, second]
    );
    assert_eq!(
        query::unique_class_by_name("p/Foo", &connection).unwrap_err(),
        "More than one indexed JVM class matches p/Foo",
    );
    for name in ["Missing", "p.Foo", "p/foo", "p/F"] {
        assert!(query::find_classes(name, &connection).unwrap().is_empty());
    }
}

#[test]
fn database_errors_are_not_missing_classes() {
    let connection = database();
    connection.execute("DROP TABLE models", []).unwrap();
    assert!(query::find_classes("p/Foo", &connection).is_err());
    assert!(query::unique_class_by_name("p/Foo", &connection).is_err());
}

#[test]
fn loading_a_missing_class_returns_none_and_invalid_models_return_errors() {
    let mut connection = database();
    let location = Location::file("/Foo.class");
    assert!(
        query::unique_class_by_name("p/Foo", &connection)
            .unwrap()
            .is_none()
    );
    store("p/Foo", &location, &mut connection);
    assert_eq!(
        query::unique_class_by_name("p/Foo", &connection)
            .unwrap()
            .unwrap(),
        class_model("p/Foo")
    );

    connection
        .execute(r#"UPDATE models SET model = '{"name":"p/Foo"}'"#, [])
        .unwrap();
    assert!(query::unique_class_by_name("p/Foo", &connection).is_err());
}

#[test]
fn invalid_matching_locations_are_errors_but_unrelated_rows_are_not_decoded() {
    let mut connection = database();
    let location = Location::file("/Foo.class");
    store("p/Foo", &location, &mut connection);
    connection
        .execute(
            r#"INSERT INTO models (location, kind, model)
               VALUES ('{}', 0, '{"name":"p/Broken"}')"#,
            [],
        )
        .unwrap();
    assert_eq!(
        query::find_classes("p/Foo", &connection).unwrap(),
        [location]
    );
    assert!(
        query::find_classes("p/Missing", &connection)
            .unwrap()
            .is_empty()
    );
    assert!(query::find_classes("p/Broken", &connection).is_err());
}

#[test]
fn queries_borrow_the_callers_transaction_and_follow_index_replacement_and_deletion() {
    let mut connection = database();
    let location = Location::file("/Foo.class");
    store("p/Old", &location, &mut connection);
    {
        let transaction = connection.transaction().unwrap();
        storage::remove("/Foo.class".as_ref(), &transaction).unwrap();
        storage::store(&class_model("p/New"), &location, &transaction).unwrap();
        assert!(
            query::find_classes("p/Old", &transaction)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            query::find_classes("p/New", &transaction).unwrap(),
            std::slice::from_ref(&location)
        );
    }
    assert_eq!(
        query::find_classes("p/Old", &connection).unwrap(),
        std::slice::from_ref(&location)
    );
    assert!(
        query::find_classes("p/New", &connection)
            .unwrap()
            .is_empty()
    );

    let transaction = connection.transaction().unwrap();
    storage::remove("/Foo.class".as_ref(), &transaction).unwrap();
    storage::store(&class_model("p/New"), &location, &transaction).unwrap();
    transaction.commit().unwrap();
    assert!(
        query::find_classes("p/Old", &connection)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        query::find_classes("p/New", &connection).unwrap(),
        [location]
    );

    let transaction = connection.transaction().unwrap();
    storage::remove("/Foo.class".as_ref(), &transaction).unwrap();
    transaction.commit().unwrap();
    assert!(
        query::find_classes("p/New", &connection)
            .unwrap()
            .is_empty()
    );
}

fn store(name: &str, location: &Location, connection: &mut Connection) {
    let transaction = connection.transaction().unwrap();
    storage::store(&class_model(name), location, &transaction).unwrap();
    transaction.commit().unwrap();
}
