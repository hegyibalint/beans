use rusqlite::Connection;
use tempfile::tempdir;

use super::{SCHEMA_VERSION, initialize, open};

#[test]
fn startup_initializes_and_configures_the_database() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    let connection = open(&path).unwrap();

    assert!(path.is_file());
    assert_eq!(version(&connection), SCHEMA_VERSION);
    assert_eq!(count(&connection, "entries"), 0);
    assert_eq!(count(&connection, "locations"), 0);
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
fn matching_version_preserves_cached_data() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    {
        let connection = open(&path).unwrap();
        connection
            .execute("INSERT INTO entries VALUES (1, 1, 0, X'01', NULL)", [])
            .unwrap();
    }
    let connection = open(&path).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
    assert_eq!(count(&connection, "entries"), 1);
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
    assert_eq!(count(&connection, "entries"), 0);
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
        connection
            .execute_batch(
                "INSERT INTO entries VALUES (1, 1, 0, X'01', NULL);
                 INSERT INTO locations VALUES (1, 2, 'container', 'p/C.class');
                 INSERT INTO locations VALUES (1, 0, 'file', '/project/app.zip');
                 INSERT INTO locations VALUES (1, 1, 'container', 'lib/a.jar');
                 INSERT INTO entries VALUES (2, 1, 1, X'02', X'03');
                 INSERT INTO locations VALUES (2, 0, 'virtual', 'UNTITLED:My%20Type.java');",
            )
            .unwrap();
    }
    let connection = open(&path).unwrap();
    let mut statement = connection
        .prepare("SELECT kind, value FROM locations WHERE entry_id = 1 ORDER BY position")
        .unwrap();
    let locations = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        locations,
        vec![
            ("file".into(), "/project/app.zip".into()),
            ("container".into(), "lib/a.jar".into()),
            ("container".into(), "p/C.class".into()),
        ]
    );
    let uri: String = connection
        .query_row(
            "SELECT value FROM locations WHERE entry_id = 2",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(uri, "UNTITLED:My%20Type.java");
}

#[test]
fn locations_require_an_owner_and_unique_nonnegative_positions() {
    let root = tempdir().unwrap();
    let connection = open(&root.path().join("models.sqlite")).unwrap();
    connection
        .execute_batch("INSERT INTO entries VALUES (1, 1, 0, X'01', NULL)")
        .unwrap();
    for (owner, position, kind) in [(2, 0, "file"), (1, -1, "file"), (1, 0, "unknown")] {
        assert!(
            connection
                .execute(
                    "INSERT INTO locations VALUES (?1, ?2, ?3, 'value')",
                    rusqlite::params![owner, position, kind],
                )
                .is_err()
        );
    }
    connection
        .execute_batch("INSERT INTO locations VALUES (1, 0, 'file', '/project/C.class')")
        .unwrap();
    assert!(
        connection
            .execute(
                "INSERT INTO locations VALUES (1, 0, 'virtual', 'untitled:C.java')",
                [],
            )
            .is_err()
    );
    connection
        .execute("DELETE FROM entries WHERE id = 1", [])
        .unwrap();
    assert_eq!(count(&connection, "locations"), 0);
}

#[test]
fn resetting_a_cache_with_linked_locations_removes_both_tables() {
    let root = tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    {
        let mut connection = open(&path).unwrap();
        connection
            .execute_batch(
                "INSERT INTO entries VALUES (1, 1, 0, X'01', NULL);
             INSERT INTO locations VALUES (1, 0, 'file', '/project/C.class');",
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", SCHEMA_VERSION - 1)
            .unwrap();
        initialize(&mut connection, include_str!("schema.sql")).unwrap();
        assert_eq!(count(&connection, "entries"), 0);
        assert_eq!(count(&connection, "locations"), 0);
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
