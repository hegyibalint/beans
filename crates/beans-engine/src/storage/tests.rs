use std::{path::Path, thread};

use beans_core::resource::{ResourceEntry, ResourceId};
use rusqlite::{Connection, OpenFlags, params};
use tempfile::tempdir;

use super::open;

#[test]
fn jvm_publication_rolls_back_every_row_when_an_insert_fails() {
    let root = tempdir().unwrap();
    let mut storage = super::Storage::new(&root.path().join("index.sqlite")).unwrap();
    let source = ResourceId::file("runtime.jar").entry(ResourceEntry("p/C.class".into()));
    let url = source.to_url().unwrap();
    let model =
        beans_platform_jvm::codec::encode(&beans_platform_jvm::Class { name: "p/C".into() })
            .unwrap();
    assert!(
        storage
            .publish_jvm(&[(url.clone(), model.clone()), (url.clone(), model.clone())])
            .is_err()
    );
    assert_eq!(entry_count(&storage.writer), 0);
    assert_eq!(
        storage.publish_jvm(&[(url, model)]).unwrap(),
        Some(beans_core::revision::Revision::new(1))
    );
    assert_eq!(entry_count(&storage.writer), 1);
}

#[test]
fn owned_storage_publishes_atomically_while_reader_snapshots_remain_pinned() {
    let root = tempdir().unwrap();
    let mut storage = super::Storage::new(&root.path().join("index.sqlite")).unwrap();
    let source = ResourceId::file("Example.java");
    let old = "package old; class Example {}";
    let new = "package new; class Example {}";
    let old_file = beans_lang_java::process(&source, old).unwrap();
    let new_file = beans_lang_java::process(&source, new).unwrap();
    assert_eq!(
        storage.publish_java(&source, &old_file, old).unwrap(),
        beans_core::revision::Revision::new(1)
    );
    let mut reader = storage.reader().unwrap();
    assert_eq!(pragma::<i32>(&reader, "busy_timeout"), 5000);
    assert_eq!(pragma::<i32>(&reader, "synchronous"), 2);
    assert!(reader.execute("DELETE FROM entries", []).is_err());
    let snapshot = super::ModelQuery::new(&mut reader).unwrap();
    storage.writer.execute_batch("CREATE TEMP TRIGGER fail_publication BEFORE INSERT ON entries BEGIN SELECT RAISE(ABORT, 'publication failed'); END;").unwrap();
    assert!(storage.publish_java(&source, &new_file, new).is_err());
    assert_eq!(entry_count(&storage.writer), 1);
    storage
        .writer
        .execute_batch("DROP TRIGGER fail_publication")
        .unwrap();
    assert_eq!(
        storage.publish_java(&source, &new_file, new).unwrap(),
        beans_core::revision::Revision::new(2)
    );
    use beans_lang_java::model_query::JavaModelQuery;
    assert_eq!(
        snapshot
            .java_file(&source)
            .unwrap()
            .unwrap()
            .package_name
            .as_slice(),
        ["old"]
    );
    assert_eq!(snapshot.content(&source).unwrap().unwrap(), old.as_bytes());
    drop(snapshot);
    let latest = super::ModelQuery::new(&mut reader).unwrap();
    assert_eq!(
        latest
            .java_file(&source)
            .unwrap()
            .unwrap()
            .package_name
            .as_slice(),
        ["new"]
    );
    assert_eq!(latest.content(&source).unwrap().unwrap(), new.as_bytes());
}

#[test]
fn owned_storage_rejects_publication_after_revision_overflow() {
    let root = tempdir().unwrap();
    let mut storage = super::Storage::new(&root.path().join("index.sqlite")).unwrap();
    let source = ResourceId::file("Example.java");
    let text = "class Example {}";
    let file = beans_lang_java::process(&source, text).unwrap();
    storage.publish_java(&source, &file, text).unwrap();
    storage
        .writer
        .execute("UPDATE entries SET revision = ?1", [i64::MAX])
        .unwrap();
    assert_eq!(
        storage.publish_java(&source, &file, text).unwrap_err(),
        "revision overflow"
    );
    assert_eq!(entry_count(&storage.writer), 1);
}

#[test]
fn startup_creates_one_table_and_configures_the_connection() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let connection = open(&path).unwrap();

    assert!(path.is_file());
    assert_eq!(pragma::<String>(&connection, "journal_mode"), "wal");
    assert_eq!(pragma::<i32>(&connection, "synchronous"), 2);
    assert_eq!(pragma::<i32>(&connection, "busy_timeout"), 5000);
    assert_eq!(pragma::<i32>(&connection, "user_version"), 1);
    assert_eq!(entry_count(&connection), 0);
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .unwrap();
    let tables: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(tables, ["entries"]);
}

#[test]
fn reopening_preserves_opaque_models_content_and_resource_urls() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let jvm = ResourceId::file("library.jar")
        .entry(ResourceEntry("p/Example.class".into()))
        .to_url()
        .unwrap();
    let java = ResourceId::file("Example.java").to_url().unwrap();
    let model = [0, 255, 1, 128];
    let content = b"class Example {}";
    {
        let connection = open(&path).unwrap();
        insert(&connection, &jvm, 1, 0, Some(&model), None).unwrap();
        insert(&connection, &java, 2, 1, Some(&model), Some(content)).unwrap();
    }

    let connection = open(&path).unwrap();
    assert_eq!(entry_count(&connection), 2);
    for (url, kind, content) in [(&jvm, 0, None), (&java, 1, Some(content.as_slice()))] {
        let (stored_kind, stored_model, stored_content): (i32, Vec<u8>, Option<Vec<u8>>) =
            connection
                .query_row(
                    "SELECT model_kind, model, content FROM entries WHERE resource_url = ?1",
                    [url],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .unwrap();
        assert_eq!(stored_kind, kind);
        assert_eq!(stored_model, model);
        assert_eq!(stored_content.as_deref(), content);
    }
    assert_eq!(pragma::<String>(&connection, "journal_mode"), "wal");
    assert_eq!(pragma::<i32>(&connection, "synchronous"), 2);
    assert_eq!(pragma::<i32>(&connection, "user_version"), 1);
}

#[test]
fn schema_enforces_kind_content_and_required_model_constraints() {
    let root = tempdir().unwrap();
    let connection = open(&root.path().join("index.sqlite")).unwrap();
    let url = "beans:file:Example";
    let model = b"opaque".as_slice();

    assert!(insert(&connection, url, 1, 0, Some(model), None).is_ok());
    assert!(insert(&connection, url, 2, 1, Some(model), Some(b"source")).is_ok());
    assert!(insert(&connection, url, 3, 1, Some(model), None).is_err());
    assert!(insert(&connection, url, 3, 2, Some(model), None).is_err());
    assert!(insert(&connection, url, 3, 0, None, None).is_err());
    assert!(insert(&connection, url, -1, 0, Some(model), None).is_err());
    assert!(insert(&connection, url, 1, 0, Some(model), None).is_err());
    assert_eq!(entry_count(&connection), 2);
}

#[test]
fn schema_requires_blob_payloads() {
    let root = tempdir().unwrap();
    let connection = open(&root.path().join("index.sqlite")).unwrap();

    assert!(
        connection
            .execute(
                "INSERT INTO entries VALUES ('beans:file:Example', 1, 0, 'text', NULL)",
                [],
            )
            .is_err()
    );
    assert!(
        connection
            .execute(
                "INSERT INTO entries VALUES ('beans:file:Example', 1, 1, X'00', 'text')",
                [],
            )
            .is_err()
    );
    assert_eq!(entry_count(&connection), 0);
}

#[test]
fn dropping_a_publication_transaction_rolls_back_all_its_rows() {
    let root = tempdir().unwrap();
    let mut connection = open(&root.path().join("index.sqlite")).unwrap();
    {
        let transaction = connection.transaction().unwrap();
        insert(&transaction, "beans:file:First", 1, 0, Some(b"first"), None).unwrap();
        insert(
            &transaction,
            "beans:file:Second",
            1,
            0,
            Some(b"second"),
            None,
        )
        .unwrap();
    }
    assert_eq!(entry_count(&connection), 0);
}

#[test]
fn a_wal_reader_retains_its_snapshot_while_the_writer_commits() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    let mut writer = open(&path).unwrap();
    let mut reader = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let snapshot = reader.transaction().unwrap();
    assert_eq!(entry_count(&snapshot), 0);

    let _writer = thread::spawn(move || {
        let transaction = writer.transaction().unwrap();
        insert(&transaction, "beans:file:First", 1, 0, Some(b"first"), None).unwrap();
        insert(
            &transaction,
            "beans:file:Second",
            1,
            0,
            Some(b"second"),
            None,
        )
        .unwrap();
        transaction.commit().unwrap();
        writer
    })
    .join()
    .unwrap();

    assert_eq!(entry_count(&snapshot), 0);
    snapshot.commit().unwrap();
    assert_eq!(entry_count(&reader), 2);
}

#[test]
fn historical_lookup_selects_the_version_before_inspecting_kind() {
    let root = tempdir().unwrap();
    let connection = open(&root.path().join("index.sqlite")).unwrap();
    let url = "beans:file:Example";
    insert(&connection, url, 1, 0, Some(b"old jvm"), None).unwrap();
    insert(&connection, url, 2, 1, Some(b"new java"), Some(b"source")).unwrap();

    for (revision, expected_kind, expected_model) in [
        (1, 0, b"old jvm".as_slice()),
        (2, 1, b"new java".as_slice()),
    ] {
        let (kind, model): (i32, Vec<u8>) = connection
            .query_row(
                "SELECT model_kind, model FROM entries
                 WHERE resource_url = ?1 AND revision <= ?2
                 ORDER BY revision DESC LIMIT 1",
                params![url, revision],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(kind, expected_kind);
        assert_eq!(model, expected_model);
    }
}

#[test]
fn unsupported_schema_versions_are_rejected_without_resetting_data() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE sentinel (value INTEGER); INSERT INTO sentinel VALUES (42)",
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 2).unwrap();
    }

    assert_eq!(
        open(&path).unwrap_err(),
        "unsupported SQLite schema version: 2"
    );
    let connection = Connection::open(&path).unwrap();
    assert_eq!(pragma::<i32>(&connection, "user_version"), 2);
    assert_eq!(
        connection
            .query_row("SELECT value FROM sentinel", [], |row| row.get::<_, i32>(0))
            .unwrap(),
        42
    );
}

#[test]
fn a_nonempty_unversioned_database_is_not_claimed_or_modified() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE entries (sentinel INTEGER)")
            .unwrap();
    }

    assert_eq!(
        open(&path).unwrap_err(),
        "cannot initialize a nonempty unversioned SQLite database"
    );
    let connection = Connection::open(&path).unwrap();
    assert_eq!(pragma::<i32>(&connection, "user_version"), 0);
    connection.prepare("SELECT sentinel FROM entries").unwrap();
}

#[test]
fn a_known_version_without_the_expected_table_is_rejected() {
    let root = tempdir().unwrap();
    let path = root.path().join("index.sqlite");
    {
        let connection = Connection::open(&path).unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
    }

    assert!(open(&path).is_err());
    let connection = Connection::open(&path).unwrap();
    assert_eq!(pragma::<i32>(&connection, "user_version"), 1);
}

#[test]
fn startup_does_not_create_missing_parent_directories() {
    let root = tempdir().unwrap();
    let path = root.path().join("missing/index.sqlite");

    assert!(open(&path).is_err());
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn startup_requires_a_file_backed_wal_database() {
    assert!(
        open(Path::new(":memory:"))
            .unwrap_err()
            .contains("got memory")
    );
}

fn pragma<T: rusqlite::types::FromSql>(connection: &Connection, name: &str) -> T {
    connection
        .pragma_query_value(None, name, |row| row.get(0))
        .unwrap()
}

fn entry_count(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))
        .unwrap()
}

fn insert(
    connection: &Connection,
    url: &str,
    revision: i64,
    kind: i32,
    model: Option<&[u8]>,
    content: Option<&[u8]>,
) -> rusqlite::Result<usize> {
    connection.execute(
        "INSERT INTO entries (resource_url, revision, model_kind, model, content)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![url, revision, kind, model, content],
    )
}
