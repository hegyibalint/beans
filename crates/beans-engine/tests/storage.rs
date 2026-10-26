use beans_core::location::{Location, LocationEntry};
use beans_engine::storage::{initialize, open};
use beans_testing::{TempDir, database};
use rusqlite::Connection;

#[test]
fn startup_initializes_and_configures_the_database() {
    let files = TempDir::new();
    let path = files.path("models.sqlite");
    let connection = open(&path).unwrap();

    assert!(path.is_file());
    assert!(version(&connection) > 0);
    assert_eq!(version(&connection), version(&database()));
    assert_eq!(count(&connection, "models"), 0);
    assert_eq!(count(&connection, "compilation_scopes"), 0);
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
    let files = TempDir::new();
    let connection = open(&files.path("models.sqlite")).unwrap();
    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN
             SELECT location FROM models
             WHERE kind = 0 AND json_extract(model, '$.name') = ?1",
            ["p/Example"],
            |row| row.get(3),
        )
        .unwrap();

    assert!(plan.starts_with("SEARCH models"), "{plan}");
    assert!(plan.contains("jvm_classes_by_name"), "{plan}");
}

#[test]
fn jvm_resource_removal_uses_the_root_index() {
    let files = TempDir::new();
    let connection = open(&files.path("models.sqlite")).unwrap();
    let location = serde_json::to_string(&LocationEntry::File("/library.jar".into())).unwrap();
    let plan: String = connection
        .query_row(
            "EXPLAIN QUERY PLAN
             DELETE FROM models
             WHERE kind = 0 AND json_extract(location, '$[0]') = ?1",
            [location],
            |row| row.get(3),
        )
        .unwrap();

    assert!(plan.starts_with("SEARCH models"), "{plan}");
    assert!(plan.contains("jvm_models_by_root"), "{plan}");
}

#[test]
fn compilation_scope_names_can_repeat_within_an_import() {
    let connection = database();
    connection
        .execute_batch(
            "INSERT INTO compilation_scopes (kind, root, name, classpath)
             VALUES (1, '/repo/build-a', 'main', '[]'),
                    (1, '/repo/build-a', 'main', '[]');",
        )
        .unwrap();
    assert_eq!(count(&connection, "compilation_scopes"), 2);
}

#[test]
fn replacing_scopes_is_isolated_by_kind_and_root() {
    let mut connection = database();
    connection
        .execute_batch(
            "INSERT INTO compilation_scopes (kind, root, name, classpath)
             VALUES (1, '/repo/build-a', 'main', '[]'),
                    (1, '/repo/build-a', 'test', '[]'),
                    (1, '/repo/build-b', 'main', '[]'),
                    (2, '/repo/build-a', 'main', '[]');",
        )
        .unwrap();

    let transaction = connection.transaction().unwrap();
    transaction
        .execute_batch(
            "DELETE FROM compilation_scopes WHERE kind = 1 AND root = '/repo/build-a';
             INSERT INTO compilation_scopes (kind, root, name, classpath)
             VALUES (1, '/repo/build-a', 'integrationTest', '[]');",
        )
        .unwrap();
    transaction.commit().unwrap();

    let scopes: Vec<(i64, String, String)> = connection
        .prepare("SELECT kind, root, name FROM compilation_scopes ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        scopes,
        [
            (1, "/repo/build-b".into(), "main".into()),
            (2, "/repo/build-a".into(), "main".into()),
            (1, "/repo/build-a".into(), "integrationTest".into()),
        ]
    );
}

#[test]
fn matching_version_preserves_cached_data() {
    let files = TempDir::new();
    let path = files.path("models.sqlite");
    let expected_version = {
        let connection = open(&path).unwrap();
        connection
            .execute("INSERT INTO models VALUES (1, '[]', 0, '{}', NULL)", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO compilation_scopes (id, kind, root, name, classpath)
                 VALUES (7, 0, '/project', 'main', '[]')",
                [],
            )
            .unwrap();
        version(&connection)
    };
    let connection = open(&path).unwrap();
    assert_eq!(version(&connection), expected_version);
    assert_eq!(count(&connection, "models"), 1);
    let scope: (i64, i64, String, String, String) = connection
        .query_row(
            "SELECT id, kind, root, name, classpath FROM compilation_scopes",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(scope, (7, 0, "/project".into(), "main".into(), "[]".into()));
}

#[test]
fn blob_model_caches_are_reset_instead_of_migrated() {
    let files = TempDir::new();
    let path = files.path("models.sqlite");
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
    assert_eq!(version(&connection), version(&database()));
    assert_eq!(count(&connection, "models"), 0);
    connection
        .execute("INSERT INTO models VALUES (1, '[]', 0, '{}', NULL)", [])
        .unwrap();
}

#[test]
fn older_version_discards_the_previous_schema_and_data() {
    let files = TempDir::new();
    let path = files.path("models.sqlite");
    let expected_version = version(&database());
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "user_version", expected_version - 1)
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
    assert_eq!(version(&connection), expected_version);
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
    let files = TempDir::new();
    let path = files.path("models.sqlite");
    let future_version = version(&database()) + 1;
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE sentinel (value INTEGER); INSERT INTO sentinel VALUES (42)",
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", future_version)
            .unwrap();
    }
    assert_eq!(
        open(&path).unwrap_err(),
        format!("unsupported SQLite schema version: {future_version}")
    );
    let connection = Connection::open(&path).unwrap();
    assert_eq!(version(&connection), future_version);
    assert_eq!(count(&connection, "sentinel"), 1);
    assert_eq!(
        connection
            .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
            .unwrap(),
        "delete"
    );
}

#[test]
fn opening_does_not_create_parent_directories() {
    let files = TempDir::new();
    let path = files.path("missing/models.sqlite");
    assert!(open(&path).is_err());
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn locations_preserve_order_kinds_and_values_after_reopening() {
    let files = TempDir::new();
    let path = files.path("models.sqlite");
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
fn resetting_a_component_location_cache_discards_its_tables_and_data() {
    let files = TempDir::new();
    let path = files.path("models.sqlite");
    let expected_version = version(&database());
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
            .pragma_update(None, "user_version", expected_version - 1)
            .unwrap();
        initialize(&mut connection).unwrap();
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
        assert_eq!(version(&connection), expected_version);
    }
    assert_eq!(version(&open(&path).unwrap()), expected_version);
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
