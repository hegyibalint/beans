use beans_core::location::{Location, LocationEntry};
use rusqlite::Connection;
use tempfile::tempdir;

use super::{SCHEMA_VERSION, initialize, open, schema};

#[test]
fn startup_initializes_and_configures_the_database() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    let connection = open(&path).unwrap();

    assert!(path.is_file());
    assert_eq!(version(&connection), SCHEMA_VERSION);
    assert_eq!(count(&connection, "models"), 0);
    let obsolete_tables: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE name IN ('entries', 'locations')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(obsolete_tables, 0);
    let indexes: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_index_list('models')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(indexes, 2);
    assert_eq!(
        connection
            .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    for (pragma, expected) in [
        ("busy_timeout", 5000),
        ("synchronous", 2),
        ("foreign_keys", 1),
    ] {
        assert_eq!(
            connection
                .pragma_query_value(None, pragma, |row| row.get::<_, i64>(0))
                .unwrap(),
            expected
        );
    }
}

#[test]
fn jvm_class_lookup_uses_the_name_index() {
    let root = tempdir().unwrap();
    let connection = open(&root.path().join("models.sqlite")).unwrap();
    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN
             SELECT location FROM models
             WHERE model_kind = 0 AND json_extract(model, '$.name') = ?1",
            ["p/Example"],
            |row| row.get(3),
        )
        .unwrap();

    assert!(plan.starts_with("SEARCH models"), "{plan}");
    assert!(plan.contains("jvm_classes_by_name"), "{plan}");
}

#[test]
fn jvm_resource_removal_uses_the_root_index() {
    let root = tempdir().unwrap();
    let connection = open(&root.path().join("models.sqlite")).unwrap();
    let location = serde_json::to_string(&LocationEntry::File("/library.jar".into())).unwrap();
    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN
             DELETE FROM models
             WHERE model_kind = 0 AND json_extract(location, '$[0]') = ?1",
            [location],
            |row| row.get(3),
        )
        .unwrap();

    assert!(plan.starts_with("SEARCH models"), "{plan}");
    assert!(plan.contains("jvm_models_by_root"), "{plan}");
}

#[test]
fn matching_version_preserves_cached_data() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    {
        let connection = open(&path).unwrap();
        connection
            .execute("INSERT INTO models VALUES (1, '[]', 0, '{}', NULL)", [])
            .unwrap();
    }
    let connection = open(&path).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
    assert_eq!(count(&connection, "models"), 1);
}

#[test]
fn model_payloads_must_be_valid_json_text() {
    let root = tempdir().unwrap();
    let connection = open(&root.path().join("models.sqlite")).unwrap();
    for model in ["'not JSON'", "'{broken}'", "X'7B7D'", "NULL"] {
        assert!(
            connection
                .execute(
                    &format!("INSERT INTO models VALUES (1, '[]', 0, {model}, NULL)"),
                    [],
                )
                .is_err()
        );
    }
    assert_eq!(count(&connection, "models"), 0);
}

#[test]
fn blob_model_caches_are_reset_instead_of_migrated() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE entries (id INTEGER PRIMARY KEY, model BLOB NOT NULL) STRICT;
             INSERT INTO entries VALUES (1, X'01');
             PRAGMA user_version = 2;",
            )
            .unwrap();
    }
    let connection = open(&path).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
    assert_eq!(count(&connection, "models"), 0);
    connection
        .execute("INSERT INTO models VALUES (1, '[]', 0, '{}', NULL)", [])
        .unwrap();
}

#[test]
fn older_version_discards_the_previous_schema_and_data() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "user_version", SCHEMA_VERSION - 1)
            .unwrap();
        connection
            .execute_batch(
                "CREATE TABLE \"old\"\"cache\" (value INTEGER);
                 INSERT INTO \"old\"\"cache\" VALUES (42);
                 CREATE INDEX old_index ON \"old\"\"cache\" (value);
                 CREATE VIEW old_view AS SELECT * FROM \"old\"\"cache\";
                 CREATE TABLE entries (obsolete TEXT);",
            )
            .unwrap();
    }
    let connection = open(&path).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
    assert_eq!(count(&connection, "models"), 0);
    let objects: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE name LIKE 'old%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(objects, 0);
}

#[test]
fn newer_version_is_rejected_without_changing_the_database() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE sentinel (value INTEGER); INSERT INTO sentinel VALUES (42)",
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
    }
    assert_eq!(
        open(&path).unwrap_err(),
        format!("unsupported SQLite schema version: {}", SCHEMA_VERSION + 1)
    );
    let connection = Connection::open(&path).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION + 1);
    assert_eq!(count(&connection, "sentinel"), 1);
    assert_eq!(
        connection
            .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
            .unwrap(),
        "delete"
    );
}

#[test]
fn failed_initialization_rolls_back_the_reset() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("CREATE TABLE sentinel (value INTEGER); INSERT INTO sentinel VALUES (42)")
        .unwrap();
    assert!(initialize(&mut connection, "CREATE TABLE broken (").is_err());
    assert_eq!(version(&connection), 0);
    assert_eq!(count(&connection, "sentinel"), 1);
}

#[test]
fn opening_does_not_create_parent_directories() {
    let root = tempdir().unwrap();
    let path = root.path().join("missing/models.sqlite");
    assert!(open(&path).is_err());
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn locations_preserve_order_kinds_and_values_after_reopening() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    {
        let connection = open(&path).unwrap();
        connection.execute_batch(
            r#"INSERT INTO models VALUES (1, '[{"File":"/project/app.zip"},{"Container":"lib/a.jar"},{"Container":"p/C.class"}]', 0, '{}', NULL);
               INSERT INTO models VALUES (2, '[{"Virtual":"UNTITLED:My%20Type.java"}]', 1, '{}', X'03');"#,
        ).unwrap();
    }
    let connection = open(&path).unwrap();
    let location: String = connection
        .query_row("SELECT location FROM models WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    let location: Location = serde_json::from_str(&location).unwrap();
    assert_eq!(
        location,
        Location::file("/project/app.zip")
            .entry(LocationEntry::Container("lib/a.jar".into()))
            .entry(LocationEntry::Container("p/C.class".into()))
    );
    let uri: String = connection
        .query_row(
            "SELECT json_extract(location, '$[0].Virtual') FROM models WHERE id = 2",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(uri, "UNTITLED:My%20Type.java");
}

#[test]
fn locations_must_be_valid_json_text() {
    let root = tempdir().unwrap();
    let connection = open(&root.path().join("models.sqlite")).unwrap();
    for location in ["'not JSON'", "'{broken}'", "X'5B5D'", "NULL"] {
        assert!(
            connection
                .execute(
                    &format!("INSERT INTO models VALUES (1, {location}, 0, '{{}}', NULL)"),
                    [],
                )
                .is_err()
        );
    }
    assert_eq!(count(&connection, "models"), 0);
}

#[test]
fn resetting_a_component_location_cache_discards_its_tables_and_data() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    {
        let mut connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        connection.execute_batch(
            "CREATE TABLE entries (id INTEGER PRIMARY KEY, model_kind INTEGER, model TEXT, content BLOB) STRICT;
             CREATE TABLE locations (entry_id INTEGER REFERENCES entries(id) ON DELETE CASCADE, position INTEGER, kind TEXT, value TEXT) STRICT;
             CREATE TABLE java_types (entry_id INTEGER REFERENCES entries(id) ON DELETE CASCADE, node_index INTEGER, name TEXT) STRICT;
             INSERT INTO entries VALUES (1, 1, '{}', X'03');
             INSERT INTO locations VALUES (1, 0, 'file', '/project/C.java');
             INSERT INTO java_types VALUES (1, 1, 'C');",
        ).unwrap();
        connection
            .pragma_update(None, "user_version", SCHEMA_VERSION - 1)
            .unwrap();
        initialize(&mut connection, &schema()).unwrap();
        assert_eq!(count(&connection, "models"), 0);
        assert_eq!(count(&connection, "java_types"), 0);
        let obsolete_tables: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE name IN ('entries', 'locations')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(obsolete_tables, 0);
    }
    assert_eq!(version(&open(&path).unwrap()), SCHEMA_VERSION);
}

fn version(connection: &Connection) -> i64 {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}
