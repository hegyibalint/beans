use beans_core::{
    classpath::Unrestricted,
    location::{Location, LocationEntry},
    names::Name,
    query_scope::QueryScope,
};
use beans_engine::storage::open;
use beans_lang_java::{
    lower_into,
    query::JavaQuery,
    storage::{remove, store},
};
use beans_testing::TempDir;
use rusqlite::Connection;

#[test]
fn java_models_are_stored_as_queryable_json_without_consuming_the_model() {
    let root = TempDir::new();
    let path = root.path("models.sqlite");
    let contents = "package p.q; import java.util.List; public class Box<T extends Number> extends Base implements Named { private List<? super T[]> values; class Nested {} }";
    let file = lower_into(contents);
    let location = Location::file("/project/sources.zip")
        .entry(LocationEntry::Container("lib/sources.jar".into()))
        .entry(LocationEntry::Container("p/q/Box.java".into()));
    let id = {
        let mut connection = open(&path).unwrap();
        let transaction = connection.transaction().unwrap();
        let id = store(&transaction, &file, &location, contents).unwrap();
        transaction.commit().unwrap();
        id
    };
    let connection = open(&path).unwrap();
    let (kind, model, content): (i64, String, Vec<u8>) = connection
        .query_row(
            "SELECT kind, model, content FROM models WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(kind, 1);
    assert_eq!(content, contents.as_bytes());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&model).unwrap(),
        serde_json::to_value(&file).unwrap()
    );
    let (storage_type, package, name): (String, String, String) = connection
        .query_row(
            "SELECT typeof(model), json_extract(model, '$.package_name.components[0]'),
                json_extract(model, '$.nodes[1].kind.Type.name.value')
         FROM models WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(storage_type, "text");
    assert_eq!(package, "p");
    assert_eq!(name, "Box");
    let (storage_type, stored_location): (String, String) = connection
        .query_row(
            "SELECT typeof(location), location FROM models WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(storage_type, "text");
    assert_eq!(
        serde_json::from_str::<Location>(&stored_location).unwrap(),
        location
    );
    assert_eq!(file.find_type(&name_of("p.q.Box.Nested")).len(), 1);
}

#[test]
fn models_can_be_selected_by_the_first_location_component() {
    let root = TempDir::new();
    let mut connection = open(&root.path("models.sqlite")).unwrap();
    let transaction = connection.transaction().unwrap();
    let file = lower_into("class Example {}");
    for source in [
        Location::file("/sources.jar").entry(LocationEntry::Container("A.java".into())),
        Location::file("/sources.jar").entry(LocationEntry::Container("B.java".into())),
        Location::file("/other.jar").entry(LocationEntry::Container("A.java".into())),
        Location::new(LocationEntry::Virtual("/sources.jar".into())),
        Location { entries: vec![] },
    ] {
        store(&transaction, &file, &source, "class Example {}").unwrap();
    }
    transaction.commit().unwrap();
    let mut statement = connection
        .prepare(
            "SELECT id FROM models
             WHERE json_extract(location, '$[0].File') = ?1
             ORDER BY id",
        )
        .unwrap();
    let ids = statement
        .query_map(["/sources.jar"], |row| row.get::<_, i64>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(ids, [1, 2]);
}

#[test]
fn replacing_a_model_reuses_its_row_and_preserves_the_verbatim_location() {
    let root = TempDir::new();
    let mut connection = open(&root.path("models.sqlite")).unwrap();
    let uri = "UNTITLED:My%20Type.java";
    let location = Location::new(LocationEntry::Virtual(uri.into()));
    let transaction = connection.transaction().unwrap();
    let mut ids = Vec::new();
    for contents in ["class First {}", "class Second {} // 😀"] {
        let id = store(&transaction, &lower_into(contents), &location, contents).unwrap();
        let stored: (Vec<u8>, String) = transaction
            .query_row(
                "SELECT content, json_extract(location, '$[0].Virtual') FROM models WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(stored, (contents.as_bytes().to_vec(), uri.into()));
        ids.push(id);
    }
    assert_eq!(ids[0], ids[1]);
    transaction.commit().unwrap();
    for table in ["models", "java_types"] {
        assert_eq!(count(&connection, table), 1);
    }
    let query = JavaQuery::new(&connection, QueryScope::new(&Unrestricted));
    assert!(query.find_type(&name_of("First")).unwrap().is_empty());
    assert_eq!(query.find_type(&name_of("Second")).unwrap().len(), 1);
}

#[test]
fn the_caller_controls_commit_and_rollback() {
    let root = TempDir::new();
    let mut connection = open(&root.path("models.sqlite")).unwrap();
    {
        let transaction = connection.transaction().unwrap();
        store(
            &transaction,
            &lower_into("class Example {}"),
            &Location::file("/Example.java"),
            "class Example {}",
        )
        .unwrap();
        for table in ["models", "java_types"] {
            assert_eq!(count(&transaction, table), 1);
        }
    }
    for table in ["models", "java_types"] {
        assert_eq!(count(&connection, table), 0);
    }
}

#[test]
fn publication_errors_allow_the_caller_to_roll_back_the_whole_batch() {
    let root = TempDir::new();
    let mut connection = open(&root.path("models.sqlite")).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_location BEFORE INSERT ON models WHEN json_array_length(NEW.location) > 1 BEGIN SELECT RAISE(ABORT, 'rejected location'); END;").unwrap();
    {
        let transaction = connection.transaction().unwrap();
        let contents = "class Example {}";
        let file = lower_into(contents);
        store(
            &transaction,
            &file,
            &Location::file("/Healthy.java"),
            contents,
        )
        .unwrap();
        let location =
            Location::file("/sources.jar").entry(LocationEntry::Container("Example.java".into()));
        let error = store(&transaction, &file, &location, contents).unwrap_err();
        assert!(error.contains("rejected location"));
        assert_eq!(count(&transaction, "models"), 1);
        assert_eq!(count(&transaction, "java_types"), 1);
    }
    assert_eq!(count(&connection, "models"), 0);
    assert_eq!(count(&connection, "java_types"), 0);
}

#[cfg(unix)]
#[test]
fn non_utf8_file_locations_are_rejected_instead_of_changed() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};
    let root = TempDir::new();
    let mut connection = open(&root.path("models.sqlite")).unwrap();
    {
        let transaction = connection.transaction().unwrap();
        let path = PathBuf::from(OsString::from_vec(b"/Example\xff.java".to_vec()));
        let error = store(&transaction, &lower_into(""), &Location::file(path), "").unwrap_err();
        assert!(error.contains("UTF-8"));
    }
    assert_eq!(count(&connection, "models"), 0);
}

#[test]
fn read_transactions_keep_old_models_and_indexes_after_replacement() {
    let root = TempDir::new();
    let path = root.path("models.sqlite");
    let source = Location::file("/Example.java");
    let mut writer = open(&path).unwrap();
    {
        let transaction = writer.transaction().unwrap();
        store(
            &transaction,
            &lower_into("class Former {}"),
            &source,
            "class Former {}",
        )
        .unwrap();
        transaction.commit().unwrap();
    }
    let mut reader = open(&path).unwrap();
    {
        let transaction = reader.transaction().unwrap();
        let query = JavaQuery::new(&transaction, QueryScope::new(&Unrestricted));
        let handle = query.find_type(&name_of("Former")).unwrap().remove(0);
        let publication = writer.transaction().unwrap();
        store(
            &publication,
            &lower_into("class Latest {}"),
            &source,
            "class Latest {}",
        )
        .unwrap();
        publication.commit().unwrap();
        assert_eq!(
            query.declaration(&handle).unwrap().unwrap().name(),
            Some("Former")
        );
        assert_eq!(query.find_type(&name_of("Former")).unwrap().len(), 1);
        assert!(query.find_type(&name_of("Latest")).unwrap().is_empty());
        assert_eq!(
            query
                .file(&source)
                .unwrap()
                .unwrap()
                .find_type(&name_of("Former"))
                .len(),
            1
        );
    }
    let query = JavaQuery::new(&reader, QueryScope::new(&Unrestricted));
    assert!(query.find_type(&name_of("Former")).unwrap().is_empty());
    assert_eq!(query.find_type(&name_of("Latest")).unwrap().len(), 1);
    assert_eq!(count(&writer, "models"), 1);
}

#[test]
fn deleting_a_model_keeps_existing_readers_consistent_and_removes_all_owned_rows() {
    let root = TempDir::new();
    let path = root.path("models.sqlite");
    let source = Location::file("/Example.java");
    let mut writer = open(&path).unwrap();
    {
        let transaction = writer.transaction().unwrap();
        store(
            &transaction,
            &lower_into("class Example {}"),
            &source,
            "class Example {}",
        )
        .unwrap();
        transaction.commit().unwrap();
    }
    let mut reader = open(&path).unwrap();
    {
        let transaction = reader.transaction().unwrap();
        let query = JavaQuery::new(&transaction, QueryScope::new(&Unrestricted));
        let handle = query.find_type(&name_of("Example")).unwrap().remove(0);
        let publication = writer.transaction().unwrap();
        assert!(remove(&publication, &source).unwrap());
        publication.commit().unwrap();
        for table in ["models", "java_types"] {
            assert_eq!(count(&writer, table), 0);
        }
        assert!(query.file(&source).unwrap().is_some());
        assert_eq!(
            query.declaration(&handle).unwrap().unwrap().name(),
            Some("Example")
        );
    }
    let query = JavaQuery::new(&reader, QueryScope::new(&Unrestricted));
    assert!(query.file(&source).unwrap().is_none());
    assert!(query.find_type(&name_of("Example")).unwrap().is_empty());
}

#[test]
fn readers_never_observe_a_partially_published_batch() {
    let root = TempDir::new();
    let path = root.path("models.sqlite");
    let first = Location::file("/First.java");
    let second = Location::file("/Second.java");
    let mut writer = open(&path).unwrap();
    {
        let transaction = writer.transaction().unwrap();
        store(
            &transaction,
            &lower_into("class First {}"),
            &first,
            "class First {}",
        )
        .unwrap();
        store(
            &transaction,
            &lower_into("class Second {}"),
            &second,
            "class Second {}",
        )
        .unwrap();
        transaction.commit().unwrap();
    }
    let mut reader = open(&path).unwrap();
    let publication = writer.transaction().unwrap();
    store(
        &publication,
        &lower_into("class NewFirst {}"),
        &first,
        "class NewFirst {}",
    )
    .unwrap();
    {
        let transaction = reader.transaction().unwrap();
        let query = JavaQuery::new(&transaction, QueryScope::new(&Unrestricted));
        assert_eq!(query.find_type(&name_of("First")).unwrap().len(), 1);
        assert_eq!(query.find_type(&name_of("Second")).unwrap().len(), 1);
        assert!(query.find_type(&name_of("NewFirst")).unwrap().is_empty());
        store(
            &publication,
            &lower_into("class NewSecond {}"),
            &second,
            "class NewSecond {}",
        )
        .unwrap();
        publication.commit().unwrap();
        assert_eq!(query.find_type(&name_of("First")).unwrap().len(), 1);
        assert_eq!(query.find_type(&name_of("Second")).unwrap().len(), 1);
        assert!(query.find_type(&name_of("NewFirst")).unwrap().is_empty());
        assert!(query.find_type(&name_of("NewSecond")).unwrap().is_empty());
    }
    let query = JavaQuery::new(&reader, QueryScope::new(&Unrestricted));
    assert!(query.find_type(&name_of("First")).unwrap().is_empty());
    assert!(query.find_type(&name_of("Second")).unwrap().is_empty());
    assert_eq!(query.find_type(&name_of("NewFirst")).unwrap().len(), 1);
    assert_eq!(query.find_type(&name_of("NewSecond")).unwrap().len(), 1);
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn name_of(name: &str) -> Name {
    name.split('.').map(str::to_owned).collect()
}
