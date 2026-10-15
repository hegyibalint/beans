use beans_core::{
    location::{Location, LocationEntry},
    names::Name,
    revision::Revision,
};
use beans_engine::storage::open;
use beans_lang_java::{lower_into, storage::store};
use rusqlite::Connection;

#[test]
fn java_models_are_stored_as_queryable_json_without_consuming_the_model() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("models.sqlite");
    let contents = "package p.q; import java.util.List; public class Box<T extends Number> extends Base implements Named { private List<? super T[]> values; class Nested {} }";
    let file = lower_into(contents);
    let location = Location::file("/project/sources.zip")
        .entry(LocationEntry::Container("lib/sources.jar".into()))
        .entry(LocationEntry::Container("p/q/Box.java".into()));
    let id = {
        let mut connection = open(&path).unwrap();
        let transaction = connection.transaction().unwrap();
        let id = store(&transaction, Revision::new(7), &file, &location, contents).unwrap();
        transaction.commit().unwrap();
        id
    };
    let connection = open(&path).unwrap();
    let (revision, kind, model, content): (i64, i64, String, Vec<u8>) = connection
        .query_row(
            "SELECT revision, model_kind, model, content FROM entries WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(revision, 7);
    assert_eq!(kind, 1);
    assert_eq!(content, contents.as_bytes());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&model).unwrap(),
        serde_json::to_value(&file).unwrap(),
    );
    let (storage_type, package, name): (String, String, String) = connection
        .query_row(
            "SELECT typeof(model), json_extract(model, '$.package_name.components[0]'),
                    json_extract(model, '$.nodes[1].kind.Type.name.value')
             FROM entries WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(storage_type, "text");
    assert_eq!(package, "p");
    assert_eq!(name, "Box");
    let mut statement = connection
        .prepare(
            "SELECT position, kind, value FROM locations WHERE entry_id = ?1 ORDER BY position",
        )
        .unwrap();
    let locations = statement
        .query_map([id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        locations,
        vec![
            (0, "file".into(), "/project/sources.zip".into()),
            (1, "container".into(), "lib/sources.jar".into()),
            (2, "container".into(), "p/q/Box.java".into()),
        ]
    );
    let name = Name::new(vec!["p".into(), "q".into(), "Box".into(), "Nested".into()]);
    assert_eq!(file.find_type(&name).len(), 1);
}

#[test]
fn each_model_version_has_its_own_id_and_verbatim_virtual_location() {
    let root = tempfile::tempdir().unwrap();
    let mut connection = open(&root.path().join("models.sqlite")).unwrap();
    let uri = "UNTITLED:My%20Type.java";
    let location = Location::new(LocationEntry::Virtual(uri.into()));
    let transaction = connection.transaction().unwrap();
    let mut ids = Vec::new();
    for (revision, contents) in [(1, "class First {}"), (2, "class Second {} // 😀")] {
        let file = lower_into(contents);
        let id = store(
            &transaction,
            Revision::new(revision),
            &file,
            &location,
            contents,
        )
        .unwrap();
        let stored: (i64, Vec<u8>, String) = transaction.query_row(
            "SELECT revision, content, value FROM entries JOIN locations ON entries.id = entry_id WHERE entries.id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert_eq!(stored, (revision, contents.as_bytes().to_vec(), uri.into()));
        ids.push(id);
    }
    assert_ne!(ids[0], ids[1]);
    transaction.commit().unwrap();
    assert_eq!(count(&connection, "entries"), 2);
    assert_eq!(count(&connection, "locations"), 2);
}

#[test]
fn the_caller_controls_commit_and_rollback() {
    let root = tempfile::tempdir().unwrap();
    let mut connection = open(&root.path().join("models.sqlite")).unwrap();
    {
        let transaction = connection.transaction().unwrap();
        store(
            &transaction,
            Revision::new(1),
            &lower_into("class Example {}"),
            &Location::file("/Example.java"),
            "class Example {}",
        )
        .unwrap();
        assert_eq!(count(&transaction, "entries"), 1);
        assert_eq!(count(&transaction, "locations"), 1);
    }
    assert_eq!(count(&connection, "entries"), 0);
    assert_eq!(count(&connection, "locations"), 0);
}

#[test]
fn location_write_errors_propagate_and_the_caller_can_roll_back_all_inserts() {
    let root = tempfile::tempdir().unwrap();
    let mut connection = open(&root.path().join("models.sqlite")).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER reject_component BEFORE INSERT ON locations WHEN NEW.position = 1
         BEGIN SELECT RAISE(ABORT, 'rejected component'); END;",
        )
        .unwrap();
    {
        let transaction = connection.transaction().unwrap();
        let location =
            Location::file("/sources.jar").entry(LocationEntry::Container("Example.java".into()));
        let error = store(
            &transaction,
            Revision::new(1),
            &lower_into(""),
            &location,
            "",
        )
        .unwrap_err();
        assert!(error.contains("rejected component"));
        assert_eq!(count(&transaction, "entries"), 1);
        assert_eq!(count(&transaction, "locations"), 1);
    }
    assert_eq!(count(&connection, "entries"), 0);
    assert_eq!(count(&connection, "locations"), 0);
}

#[test]
fn the_maximum_revision_is_stored_without_loss() {
    let root = tempfile::tempdir().unwrap();
    let mut connection = open(&root.path().join("models.sqlite")).unwrap();
    let transaction = connection.transaction().unwrap();
    let id = store(
        &transaction,
        Revision::new(i64::MAX),
        &lower_into(""),
        &Location::file("/Example.java"),
        "",
    )
    .unwrap();
    transaction.commit().unwrap();
    let revision: i64 = connection
        .query_row("SELECT revision FROM entries WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(revision, i64::MAX);
}

#[cfg(unix)]
#[test]
fn non_utf8_file_locations_are_rejected_instead_of_changed() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

    let root = tempfile::tempdir().unwrap();
    let mut connection = open(&root.path().join("models.sqlite")).unwrap();
    {
        let transaction = connection.transaction().unwrap();
        let path = PathBuf::from(OsString::from_vec(b"/Example\xff.java".to_vec()));
        let error = store(
            &transaction,
            Revision::new(1),
            &lower_into(""),
            &Location::file(path),
            "",
        )
        .unwrap_err();
        assert!(error.contains("UTF-8"));
    }
    assert_eq!(count(&connection, "entries"), 0);
    assert_eq!(count(&connection, "locations"), 0);
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}
