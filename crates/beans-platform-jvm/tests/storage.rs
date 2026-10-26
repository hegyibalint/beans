use beans_core::location::{Location, LocationEntry};
use beans_testing::database;
use rusqlite::Connection;

use beans_platform_jvm::{
    Class,
    storage::{remove, store},
};

#[test]
fn stores_borrowed_classes_and_locations_as_json() {
    let mut connection = database();
    let class = class("example/Foo");
    let location =
        Location::file("/library.jar").entry(LocationEntry::Container("example/Foo.class".into()));
    let transaction = connection.transaction().unwrap();
    let id = store(&transaction, &class, &location).unwrap();
    transaction.commit().unwrap();

    let (kind, model, source, content): (i64, String, String, Option<Vec<u8>>) = connection
        .query_row(
            "SELECT kind, model, location, content FROM models WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(kind, 0);
    assert_eq!(
        serde_json::from_str::<Class>(&model).unwrap().name,
        class.name
    );
    assert_eq!(serde_json::from_str::<Location>(&source).unwrap(), location);
    assert!(content.is_none());
}

#[test]
fn storage_only_inserts_and_leaves_replacement_to_the_caller() {
    let mut connection = database();
    let location = Location::file("/Foo.class");
    let transaction = connection.transaction().unwrap();
    store(&transaction, &class("example/Old"), &location).unwrap();
    store(&transaction, &class("example/New"), &location).unwrap();
    transaction.commit().unwrap();

    assert_eq!(names(&connection), ["example/New", "example/Old"]);
}

#[test]
fn removal_is_scoped_to_the_physical_resource_and_jvm_models() {
    let mut connection = database();
    let root = "/library.jar";
    let transaction = connection.transaction().unwrap();
    for location in [
        Location::file(root),
        Location::file(root).entry(LocationEntry::Container("A.class".into())),
        Location::file(root)
            .entry(LocationEntry::Container("nested.jar".into()))
            .entry(LocationEntry::Container("B.class".into())),
    ] {
        store(&transaction, &class("Removed"), &location).unwrap();
    }
    let survivors = [
        Location::file("/library.jar-other"),
        Location::new(LocationEntry::Virtual(root.into())),
        Location::new(LocationEntry::Container(root.into())),
        Location::file("/outer.jar").entry(LocationEntry::Container("library.jar".into())),
        Location { entries: vec![] },
    ];
    for (index, location) in survivors.iter().enumerate() {
        store(&transaction, &class(&format!("Kept{index}")), location).unwrap();
    }
    transaction
        .execute(
            "INSERT INTO models (location, kind, model, content) VALUES (?1, 1, '{}', X'')",
            [serde_json::to_string(&Location::file(root)).unwrap()],
        )
        .unwrap();

    assert!(remove(&transaction, root.as_ref()).unwrap());
    assert!(!remove(&transaction, root.as_ref()).unwrap());
    transaction.commit().unwrap();

    assert_eq!(
        names(&connection),
        ["Kept0", "Kept1", "Kept2", "Kept3", "Kept4"]
    );
    let java_models: i64 = connection
        .query_row("SELECT COUNT(*) FROM models WHERE kind = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(java_models, 1);
}

#[test]
fn deletion_can_be_committed_without_reinserting_classes() {
    let mut connection = database();
    let source = Location::file("/library.jar").entry(LocationEntry::Container("A.class".into()));
    let transaction = connection.transaction().unwrap();
    store(&transaction, &class("Old"), &source).unwrap();
    transaction.commit().unwrap();

    let transaction = connection.transaction().unwrap();
    assert!(remove(&transaction, "/library.jar".as_ref()).unwrap());
    transaction.commit().unwrap();

    assert!(names(&connection).is_empty());
}

#[test]
fn dropping_a_replacement_restores_deleted_models() {
    let mut connection = database();
    let source = Location::file("/library.jar").entry(LocationEntry::Container("A.class".into()));
    let transaction = connection.transaction().unwrap();
    store(&transaction, &class("Old"), &source).unwrap();
    transaction.commit().unwrap();

    {
        let transaction = connection.transaction().unwrap();
        assert!(remove(&transaction, "/library.jar".as_ref()).unwrap());
        assert!(names(&transaction).is_empty());
        store(&transaction, &class("New"), &source).unwrap();
        assert_eq!(names(&transaction), ["New"]);
    }
    assert_eq!(names(&connection), ["Old"]);
}

#[test]
fn a_storage_failure_allows_rolling_back_the_whole_replacement() {
    let mut connection = database();
    let source = Location::file("/library.jar").entry(LocationEntry::Container("A.class".into()));
    let transaction = connection.transaction().unwrap();
    store(&transaction, &class("Old"), &source).unwrap();
    transaction.commit().unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER reject_class BEFORE INSERT ON models
         WHEN json_extract(NEW.model, '$.name') = 'Rejected'
         BEGIN SELECT RAISE(ABORT, 'rejected class'); END;",
        )
        .unwrap();

    {
        let transaction = connection.transaction().unwrap();
        remove(&transaction, "/library.jar".as_ref()).unwrap();
        store(&transaction, &class("New"), &source).unwrap();
        let other =
            Location::file("/library.jar").entry(LocationEntry::Container("B.class".into()));
        let error = store(&transaction, &class("Rejected"), &other).unwrap_err();
        assert!(error.contains("rejected class"));
    }
    assert_eq!(names(&connection), ["Old"]);
}

fn class(name: &str) -> Class {
    Class { name: name.into() }
}

fn names(connection: &Connection) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT json_extract(model, '$.name') FROM models WHERE kind = 0 ORDER BY 1")
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}
