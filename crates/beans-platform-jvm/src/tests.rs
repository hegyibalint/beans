use std::io::{Cursor, Write};

use beans_core::location::LocationEntry;
use beans_testing::support::class_bytes;
use rusqlite::Connection;
use tempfile::NamedTempFile;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use super::*;

#[test]
fn stores_each_container_format_in_the_supplied_transaction() {
    let class = class_bytes("example/Foo");
    for (bytes, entry) in [
        (class.clone(), None),
        (
            jar_bytes(&[("example/Foo.class", &class)]),
            Some("example/Foo.class"),
        ),
        (jimage_bytes(&class), Some("/java.base/example/Foo.class")),
    ] {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&bytes).unwrap();
        let mut connection = database();
        let transaction = connection.transaction().unwrap();

        assert!(
            JvmPlatform::new()
                .process(&transaction, file.path())
                .unwrap()
        );
        transaction.commit().unwrap();

        assert_eq!(names(&connection), ["example/Foo"]);
        let source: String = connection
            .query_row("SELECT location FROM models", [], |row| row.get(0))
            .unwrap();
        let mut expected = Location::file(file.path());
        if let Some(entry) = entry {
            expected = expected.entry(LocationEntry::Container(entry.into()));
        }
        assert_eq!(serde_json::from_str::<Location>(&source).unwrap(), expected);
    }
}

#[test]
fn replacement_skips_malformed_classes_and_removes_previous_models() {
    let mut file = NamedTempFile::new().unwrap();
    std::fs::write(
        file.path(),
        jar_bytes(&[
            ("First.class", &class_bytes("Old")),
            ("Broken.class", &class_bytes("PreviouslyValid")),
            ("Missing.class", &class_bytes("Missing")),
        ]),
    )
    .unwrap();
    let mut connection = database();
    let platform = JvmPlatform::new();
    let transaction = connection.transaction().unwrap();
    assert!(platform.process(&transaction, file.path()).unwrap());
    transaction.commit().unwrap();

    let bytes = jar_bytes(&[
        ("First.class", &class_bytes("New")),
        ("Broken.class", &[0xCA, 0xFE, 0xBA, 0xBE]),
        ("Last.class", &class_bytes("Last")),
    ]);
    file.as_file_mut().set_len(0).unwrap();
    file.write_all(&bytes).unwrap();
    let transaction = connection.transaction().unwrap();
    assert!(platform.process(&transaction, file.path()).unwrap());
    transaction.commit().unwrap();

    assert_eq!(names(&connection), ["Last", "New"]);
}

#[test]
fn recognized_resources_can_replace_previous_models_with_no_classes() {
    let broken = [0xCA, 0xFE, 0xBA, 0xBE];
    for bytes in [
        jar_bytes(&[]),
        broken.to_vec(),
        jar_bytes(&[("Broken.class", &broken)]),
        jimage_bytes(&broken),
    ] {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&bytes).unwrap();
        let mut connection = database();
        seed(&mut connection, file.path());
        let transaction = connection.transaction().unwrap();

        assert!(
            JvmPlatform::new()
                .process(&transaction, file.path())
                .unwrap()
        );
        transaction.commit().unwrap();

        assert!(names(&connection).is_empty());
    }
}

#[test]
fn unknown_formats_are_declined_and_the_caller_can_restore_deleted_models() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"not a container").unwrap();
    let mut connection = database();
    seed(&mut connection, file.path());
    let transaction = connection.transaction().unwrap();

    assert!(
        !JvmPlatform::new()
            .process(&transaction, file.path())
            .unwrap()
    );
    transaction.rollback().unwrap();

    assert_eq!(names(&connection), ["Old"]);
}

#[test]
fn container_opening_errors_preserve_previous_models_on_rollback() {
    for bytes in [b"PK\x03\x04".as_slice(), &[0xDA, 0xDA, 0xFE, 0xCA]] {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        let mut connection = database();
        seed(&mut connection, file.path());
        let transaction = connection.transaction().unwrap();

        let error = JvmPlatform::new()
            .process(&transaction, file.path())
            .unwrap_err();
        assert!(error.contains(&file.path().display().to_string()));
        transaction.rollback().unwrap();

        assert_eq!(names(&connection), ["Old"]);
    }
}

#[test]
fn entry_read_errors_abort_publication_instead_of_being_skipped() {
    let good = class_bytes("First");
    let broken = class_bytes("Broken");
    let mut bytes = jar_bytes(&[("First.class", &good), ("Broken.class", &broken)]);
    let offset = bytes
        .windows(broken.len())
        .position(|bytes| bytes == broken)
        .unwrap();
    bytes[offset] ^= 1;
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    let mut connection = database();
    seed(&mut connection, file.path());
    let transaction = connection.transaction().unwrap();

    assert!(
        JvmPlatform::new()
            .process(&transaction, file.path())
            .is_err()
    );
    assert_eq!(names(&transaction), ["First"]);
    transaction.rollback().unwrap();

    assert_eq!(names(&connection), ["Old"]);
}

#[test]
fn storage_errors_abort_publication_instead_of_being_skipped() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&jar_bytes(&[
        ("First.class", &class_bytes("First")),
        ("Rejected.class", &class_bytes("Rejected")),
        ("Last.class", &class_bytes("Last")),
    ]))
    .unwrap();
    let mut connection = database();
    seed(&mut connection, file.path());
    connection
        .execute_batch(
            "CREATE TRIGGER reject_class BEFORE INSERT ON models
         WHEN json_extract(NEW.model, '$.name') = 'Rejected'
         BEGIN SELECT RAISE(ABORT, 'rejected class'); END;",
        )
        .unwrap();
    let transaction = connection.transaction().unwrap();

    let error = JvmPlatform::new()
        .process(&transaction, file.path())
        .unwrap_err();
    assert!(error.contains("rejected class"));
    assert_eq!(names(&transaction), ["First"]);
    transaction.rollback().unwrap();

    assert_eq!(names(&connection), ["Old"]);
}

#[test]
fn missing_resources_return_errors_without_losing_previous_models() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing.jar");
    let mut connection = database();
    seed(&mut connection, &path);
    let transaction = connection.transaction().unwrap();

    assert!(JvmPlatform::new().process(&transaction, &path).is_err());
    transaction.rollback().unwrap();

    assert_eq!(names(&connection), ["Old"]);
}

#[test]
fn imports_each_jdk_runtime_layout() {
    let class = class_bytes("example/Foo");
    for (relative, bytes) in [
        ("lib/modules", jimage_bytes(&class)),
        (
            "jre/lib/rt.jar",
            jar_bytes(&[("example/Foo.class", &class)]),
        ),
        ("lib/rt.jar", jar_bytes(&[("example/Foo.class", &class)])),
    ] {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        let mut connection = database();
        let transaction = connection.transaction().unwrap();
        assert_eq!(jdk_artifacts(home.path()).unwrap(), vec![path.clone()]);
        JvmPlatform::new()
            .process_jdk_home(&transaction, home.path())
            .unwrap();
        transaction.commit().unwrap();

        assert_eq!(names(&connection), ["example/Foo"]);
    }
}

#[test]
fn jdk_runtime_candidates_are_selected_in_order() {
    let home = tempfile::tempdir().unwrap();
    for relative in ["lib/rt.jar", "jre/lib/rt.jar", "lib/modules"] {
        let path = home.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, []).unwrap();
        assert_eq!(jdk_artifacts(home.path()).unwrap(), vec![path]);
    }
}

#[test]
fn invalid_existing_runtime_does_not_fall_back() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("lib")).unwrap();
    std::fs::create_dir_all(home.path().join("jre/lib")).unwrap();
    std::fs::write(home.path().join("lib/modules"), b"invalid image").unwrap();
    std::fs::write(home.path().join("jre/lib/rt.jar"), jar_bytes(&[])).unwrap();
    let mut connection = database();
    let transaction = connection.transaction().unwrap();

    let error = JvmPlatform::new()
        .process_jdk_home(&transaction, home.path())
        .unwrap_err();

    assert!(error.contains("lib/modules"));
    assert!(error.contains("unrecognized container format"));
}

#[test]
fn missing_and_non_file_runtimes_are_errors() {
    let home = tempfile::tempdir().unwrap();
    assert!(
        jdk_artifacts(home.path())
            .unwrap_err()
            .contains("no supported JDK runtime")
    );
    std::fs::create_dir_all(home.path().join("lib/modules")).unwrap();
    std::fs::write(home.path().join("lib/rt.jar"), jar_bytes(&[])).unwrap();
    assert!(
        jdk_artifacts(home.path())
            .unwrap_err()
            .contains("expected a JDK artifact file")
    );
}

#[test]
fn legacy_layouts_import_optional_tools() {
    for relative in ["jre/lib/rt.jar", "lib/rt.jar"] {
        let home = tempfile::tempdir().unwrap();
        let runtime = home.path().join(relative);
        let tools = home.path().join("lib/tools.jar");
        std::fs::create_dir_all(runtime.parent().unwrap()).unwrap();
        std::fs::create_dir_all(tools.parent().unwrap()).unwrap();
        std::fs::write(
            &runtime,
            jar_bytes(&[("Runtime.class", &class_bytes("example/Runtime"))]),
        )
        .unwrap();
        std::fs::write(
            &tools,
            jar_bytes(&[("Tools.class", &class_bytes("example/Tools"))]),
        )
        .unwrap();
        let mut connection = database();
        let transaction = connection.transaction().unwrap();
        assert_eq!(
            jdk_artifacts(home.path()).unwrap(),
            vec![runtime.clone(), tools.clone()]
        );
        JvmPlatform::new()
            .process_jdk_home(&transaction, home.path())
            .unwrap();
        transaction.commit().unwrap();

        assert_eq!(names(&connection), ["example/Runtime", "example/Tools"]);
    }
}

#[test]
fn modern_layout_ignores_legacy_artifacts() {
    let home = tempfile::tempdir().unwrap();
    let runtime = home.path().join("lib/modules");
    std::fs::create_dir_all(home.path().join("lib/tools.jar")).unwrap();
    std::fs::create_dir_all(home.path().join("jre/lib")).unwrap();
    std::fs::write(&runtime, jimage_bytes(&class_bytes("example/Foo"))).unwrap();
    std::fs::write(home.path().join("jre/lib/rt.jar"), b"invalid").unwrap();
    std::fs::write(home.path().join("lib/rt.jar"), b"invalid").unwrap();
    let mut connection = database();
    let transaction = connection.transaction().unwrap();
    assert_eq!(jdk_artifacts(home.path()).unwrap(), vec![runtime]);
    JvmPlatform::new()
        .process_jdk_home(&transaction, home.path())
        .unwrap();
    transaction.commit().unwrap();

    assert_eq!(names(&connection), ["example/Foo"]);
}

#[test]
fn tools_alone_do_not_supply_a_runtime() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("lib")).unwrap();
    std::fs::write(home.path().join("lib/tools.jar"), jar_bytes(&[])).unwrap();

    assert!(
        jdk_artifacts(home.path())
            .unwrap_err()
            .contains("no supported JDK runtime")
    );
}

#[test]
fn existing_optional_artifact_must_be_a_file() {
    for relative in ["jre/lib/rt.jar", "lib/rt.jar"] {
        let home = tempfile::tempdir().unwrap();
        let runtime = home.path().join(relative);
        std::fs::create_dir_all(runtime.parent().unwrap()).unwrap();
        std::fs::write(&runtime, jar_bytes(&[])).unwrap();
        std::fs::create_dir_all(home.path().join("lib/tools.jar")).unwrap();

        assert!(
            jdk_artifacts(home.path())
                .unwrap_err()
                .contains("expected a JDK artifact file")
        );
    }
}

#[test]
fn optional_artifact_errors_allow_rolling_back_the_jdk_publication() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("lib")).unwrap();
    std::fs::write(
        home.path().join("lib/rt.jar"),
        jar_bytes(&[("Runtime.class", &class_bytes("Runtime"))]),
    )
    .unwrap();
    std::fs::write(home.path().join("lib/tools.jar"), b"invalid tools").unwrap();
    let mut connection = database();
    let transaction = connection.transaction().unwrap();

    let error = JvmPlatform::new()
        .process_jdk_home(&transaction, home.path())
        .unwrap_err();
    assert!(error.contains("lib/tools.jar"));
    assert!(error.contains("unrecognized container format"));
    assert_eq!(names(&transaction), ["Runtime"]);
    transaction.rollback().unwrap();

    assert!(names(&connection).is_empty());
}

fn database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(include_str!("../../beans-engine/src/storage/schema.sql"))
        .unwrap();
    connection
}

fn seed(connection: &mut Connection, path: &Path) {
    let transaction = connection.transaction().unwrap();
    storage::store(
        &transaction,
        &Class { name: "Old".into() },
        &Location::file(path),
    )
    .unwrap();
    transaction.commit().unwrap();
}

fn names(connection: &Connection) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT json_extract(model, '$.name') FROM models WHERE model_kind = 0 ORDER BY 1")
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn jar_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn jimage_bytes(class: &[u8]) -> Vec<u8> {
    let strings = b"\0java.base\0example\0Foo\0class\0";
    let mut locations = vec![0, 0x08, 1, 0x10, 11, 0x18, 19, 0x20, 23, 0x3F];
    locations.extend((class.len() as u64).to_be_bytes());
    locations.push(0);

    let mut bytes = Vec::new();
    for word in [
        0xCAFEDADA,
        0x00010000,
        0,
        1,
        1,
        locations.len() as u32,
        strings.len() as u32,
        u32::MAX,
        1,
    ] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(locations);
    bytes.extend(strings);
    bytes.extend(class);
    bytes
}
