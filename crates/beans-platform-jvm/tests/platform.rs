use std::{fs, path::Path};

use beans_core::location::{Location, LocationEntry};
use beans_testing::{
    TempDir, database,
    support::{class_bytes, jar_bytes},
};
use rusqlite::Connection;

use beans_platform_jvm::{Class, JvmPlatform, jdk_artifacts, storage};

#[test]
fn stores_each_container_format_in_the_supplied_transaction() {
    let class = class_bytes("example/Foo");
    for (bytes, entry) in [
        (class.clone(), None),
        (
            jar_bytes(&[("readme.txt", b"hello"), ("example/Foo.class", &class)]),
            Some("example/Foo.class"),
        ),
        (jimage_bytes(&class), Some("/java.base/example/Foo.class")),
    ] {
        let files = TempDir::new();
        let path = files.write("resource", bytes);
        let mut connection = database();
        let platform = JvmPlatform::new();
        let processed = platform.process(&path).unwrap();
        assert_eq!(processed.classes.len(), 1);
        assert!(processed.failures.is_empty());
        assert!(names(&connection).is_empty());
        let transaction = connection.transaction().unwrap();
        platform
            .store(&transaction, &path, processed.classes)
            .unwrap();
        transaction.commit().unwrap();

        assert_eq!(names(&connection), ["example/Foo"]);
        let source: String = connection
            .query_row("SELECT location FROM models", [], |row| row.get(0))
            .unwrap();
        let mut expected = Location::file(&path);
        if let Some(entry) = entry {
            expected = expected.entry(LocationEntry::Container(entry.into()));
        }
        assert_eq!(serde_json::from_str::<Location>(&source).unwrap(), expected);
    }
}

#[test]
fn replacement_skips_malformed_classes_and_removes_previous_models() {
    let files = TempDir::new();
    let path = files.write(
        "resource",
        jar_bytes(&[
            ("First.class", &class_bytes("Old")),
            ("Broken.class", &class_bytes("PreviouslyValid")),
            ("Missing.class", &class_bytes("Missing")),
        ]),
    );
    let mut connection = database();
    let platform = JvmPlatform::new();
    let classes = platform.process(&path).unwrap().classes;
    let transaction = connection.transaction().unwrap();
    platform.store(&transaction, &path, classes).unwrap();
    transaction.commit().unwrap();

    files.write(
        "resource",
        jar_bytes(&[
            ("First.class", &class_bytes("New")),
            ("Broken.class", &[0xCA, 0xFE, 0xBA, 0xBE]),
            ("Last.class", &class_bytes("Last")),
        ]),
    );
    let processed = platform.process(&path).unwrap();
    assert_eq!(
        processed
            .classes
            .iter()
            .map(|(_, class)| class.name.as_str())
            .collect::<Vec<_>>(),
        ["New", "Last"]
    );
    assert_eq!(processed.failures.len(), 1);
    assert_eq!(
        processed.failures[0].0,
        Location::file(&path).entry(LocationEntry::Container("Broken.class".into()))
    );
    assert!(!processed.failures[0].1.is_empty());
    let transaction = connection.transaction().unwrap();
    platform
        .store(&transaction, &path, processed.classes)
        .unwrap();
    transaction.commit().unwrap();

    assert_eq!(names(&connection), ["Last", "New"]);
}

#[test]
fn empty_resources_can_replace_previous_models_without_failures() {
    for bytes in [jar_bytes(&[]), jar_bytes(&[("readme.txt", b"hello")])] {
        let files = TempDir::new();
        let path = files.write("resource", bytes);
        let mut connection = database();
        seed(&mut connection, &path);
        let platform = JvmPlatform::new();
        let processed = platform.process(&path).unwrap();
        assert!(processed.classes.is_empty());
        assert!(processed.failures.is_empty());
        let transaction = connection.transaction().unwrap();
        platform
            .store(&transaction, &path, processed.classes)
            .unwrap();
        transaction.commit().unwrap();

        assert!(names(&connection).is_empty());
    }
}

#[test]
fn invalid_classes_report_their_locations_even_when_none_succeed() {
    let broken = [0xCA, 0xFE, 0xBA, 0xBE];
    for (bytes, entry) in [
        (broken.to_vec(), None),
        (
            jar_bytes(&[("Broken.class", &broken)]),
            Some("Broken.class"),
        ),
        (
            jar_bytes(&[("Broken.class", b"not a class")]),
            Some("Broken.class"),
        ),
        (jimage_bytes(&broken), Some("/java.base/example/Foo.class")),
        (
            jimage_bytes(b"not a class"),
            Some("/java.base/example/Foo.class"),
        ),
    ] {
        let files = TempDir::new();
        let path = files.write("resource", bytes);
        let processed = JvmPlatform::new().process(&path).unwrap();
        let mut expected = Location::file(&path);
        if let Some(entry) = entry {
            expected = expected.entry(LocationEntry::Container(entry.into()));
        }

        assert!(processed.classes.is_empty());
        assert_eq!(processed.failures.len(), 1);
        assert_eq!(processed.failures[0].0, expected);
        assert!(!processed.failures[0].1.is_empty());
    }
}

#[test]
fn unsupported_formats_return_errors_without_changing_stored_models() {
    let files = TempDir::new();
    let path = files.write("resource", b"not a container");
    let mut connection = database();
    seed(&mut connection, &path);

    let error = JvmPlatform::new().process(&path).unwrap_err();
    assert!(error.contains("unsupported JVM resource format"));
    assert!(error.contains(&path.display().to_string()));
    assert_eq!(names(&connection), ["Old"]);
}

#[test]
fn invalid_containers_report_resource_failures_without_changing_stored_models() {
    for bytes in [
        b"PK\x03\x04".as_slice(),
        b"PK\x05\x06",
        b"PK\x06\x06",
        &[0xDA, 0xDA, 0xFE, 0xCA],
        &[0xCA, 0xFE, 0xDA, 0xDA],
    ] {
        let files = TempDir::new();
        let path = files.write("resource", bytes);
        let mut connection = database();
        seed(&mut connection, &path);

        let processed = JvmPlatform::new().process(&path).unwrap();
        assert!(processed.classes.is_empty());
        assert_eq!(processed.failures.len(), 1);
        assert_eq!(processed.failures[0].0, Location::file(&path));
        assert!(!processed.failures[0].1.is_empty());

        assert_eq!(names(&connection), ["Old"]);
    }
}

#[test]
fn entry_read_errors_do_not_discard_classes_or_prevent_reading_later_entries() {
    let good = class_bytes("First");
    let broken = class_bytes("Broken");
    let mut bytes = jar_bytes(&[
        ("First.class", &good),
        ("Broken.class", &broken),
        ("Last.class", &class_bytes("Last")),
    ]);
    let offset = bytes
        .windows(broken.len())
        .position(|bytes| bytes == broken)
        .unwrap();
    bytes[offset] ^= 1;
    let files = TempDir::new();
    let path = files.write("resource", bytes);
    let mut connection = database();
    seed(&mut connection, &path);

    let processed = JvmPlatform::new().process(&path).unwrap();
    assert_eq!(
        processed
            .classes
            .iter()
            .map(|(_, class)| class.name.as_str())
            .collect::<Vec<_>>(),
        ["First", "Last"]
    );
    assert_eq!(processed.failures.len(), 1);
    assert_eq!(
        processed.failures[0].0,
        Location::file(&path).entry(LocationEntry::Container("Broken.class".into()))
    );
    assert!(!processed.failures[0].1.is_empty());
    assert_eq!(names(&connection), ["Old"]);
}

#[test]
fn storage_errors_abort_publication_instead_of_being_skipped() {
    let files = TempDir::new();
    let path = files.write(
        "resource",
        jar_bytes(&[
            ("First.class", &class_bytes("First")),
            ("Rejected.class", &class_bytes("Rejected")),
            ("Last.class", &class_bytes("Last")),
        ]),
    );
    let mut connection = database();
    seed(&mut connection, &path);
    connection
        .execute_batch(
            "CREATE TRIGGER reject_class BEFORE INSERT ON models
         WHEN json_extract(NEW.model, '$.name') = 'Rejected'
         BEGIN SELECT RAISE(ABORT, 'rejected class'); END;",
        )
        .unwrap();
    let platform = JvmPlatform::new();
    let classes = platform.process(&path).unwrap().classes;
    let transaction = connection.transaction().unwrap();

    let error = platform.store(&transaction, &path, classes).unwrap_err();
    assert!(error.contains("rejected class"));
    assert_eq!(names(&transaction), ["First"]);
    transaction.rollback().unwrap();

    assert_eq!(names(&connection), ["Old"]);
}

#[test]
fn missing_resources_return_errors_without_losing_previous_models() {
    let files = TempDir::new();
    let path = files.path("missing.jar");
    let mut connection = database();
    seed(&mut connection, &path);

    assert!(JvmPlatform::new().process(&path).is_err());

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
        let home = TempDir::new();
        let path = home.write(relative, bytes);
        let mut connection = database();
        assert_eq!(jdk_artifacts(home.root()).unwrap(), vec![path.clone()]);
        let platform = JvmPlatform::new();
        let processed = platform.process(&path).unwrap();
        assert!(processed.failures.is_empty());
        let transaction = connection.transaction().unwrap();
        platform
            .store(&transaction, &path, processed.classes)
            .unwrap();
        transaction.commit().unwrap();

        assert_eq!(names(&connection), ["example/Foo"]);
    }
}

#[test]
fn jdk_runtime_candidates_are_selected_in_order() {
    let home = TempDir::new();
    for relative in ["lib/rt.jar", "jre/lib/rt.jar", "lib/modules"] {
        let path = home.write(relative, []);
        assert_eq!(jdk_artifacts(home.root()).unwrap(), vec![path]);
    }
}

#[test]
fn invalid_existing_runtime_does_not_fall_back() {
    let home = TempDir::new();
    let runtime = home.write("lib/modules", [0xDA, 0xDA, 0xFE, 0xCA]);
    home.write("jre/lib/rt.jar", jar_bytes(&[]));

    let artifacts = jdk_artifacts(home.root()).unwrap();
    assert_eq!(artifacts, [runtime]);
    let processed = JvmPlatform::new().process(&artifacts[0]).unwrap();
    assert!(processed.classes.is_empty());
    assert_eq!(processed.failures.len(), 1);
    assert_eq!(processed.failures[0].0, Location::file(&artifacts[0]));
    assert!(!processed.failures[0].1.is_empty());
}

#[test]
fn missing_and_non_file_runtimes_are_errors() {
    let home = TempDir::new();
    assert!(
        jdk_artifacts(home.root())
            .unwrap_err()
            .contains("no supported JDK runtime")
    );
    fs::create_dir_all(home.path("lib/modules")).unwrap();
    home.write("lib/rt.jar", jar_bytes(&[]));
    assert!(
        jdk_artifacts(home.root())
            .unwrap_err()
            .contains("expected a JDK artifact file")
    );
}

#[test]
fn legacy_layouts_import_optional_tools() {
    for relative in ["jre/lib/rt.jar", "lib/rt.jar"] {
        let home = TempDir::new();
        let runtime = home.write(
            relative,
            jar_bytes(&[("Runtime.class", &class_bytes("example/Runtime"))]),
        );
        let tools = home.write(
            "lib/tools.jar",
            jar_bytes(&[("Tools.class", &class_bytes("example/Tools"))]),
        );
        let artifacts = jdk_artifacts(home.root()).unwrap();
        assert_eq!(artifacts, [runtime, tools]);
        let platform = JvmPlatform::new();
        let mut connection = database();
        let transaction = connection.transaction().unwrap();
        for path in artifacts {
            let processed = platform.process(&path).unwrap();
            assert!(processed.failures.is_empty());
            platform
                .store(&transaction, &path, processed.classes)
                .unwrap();
        }
        transaction.commit().unwrap();

        assert_eq!(names(&connection), ["example/Runtime", "example/Tools"]);
    }
}

#[test]
fn modern_layout_ignores_legacy_artifacts() {
    let home = TempDir::new();
    let runtime = home.write("lib/modules", jimage_bytes(&class_bytes("example/Foo")));
    fs::create_dir_all(home.path("lib/tools.jar")).unwrap();
    home.write("jre/lib/rt.jar", b"invalid");
    home.write("lib/rt.jar", b"invalid");
    assert_eq!(jdk_artifacts(home.root()).unwrap(), vec![runtime.clone()]);
    let platform = JvmPlatform::new();
    let processed = platform.process(&runtime).unwrap();
    assert!(processed.failures.is_empty());
    let mut connection = database();
    let transaction = connection.transaction().unwrap();
    platform
        .store(&transaction, &runtime, processed.classes)
        .unwrap();
    transaction.commit().unwrap();

    assert_eq!(names(&connection), ["example/Foo"]);
}

#[test]
fn tools_alone_do_not_supply_a_runtime() {
    let home = TempDir::new();
    home.write("lib/tools.jar", jar_bytes(&[]));

    assert!(
        jdk_artifacts(home.root())
            .unwrap_err()
            .contains("no supported JDK runtime")
    );
}

#[test]
fn existing_optional_artifact_must_be_a_file() {
    for relative in ["jre/lib/rt.jar", "lib/rt.jar"] {
        let home = TempDir::new();
        home.write(relative, jar_bytes(&[]));
        fs::create_dir_all(home.path("lib/tools.jar")).unwrap();

        assert!(
            jdk_artifacts(home.root())
                .unwrap_err()
                .contains("expected a JDK artifact file")
        );
    }
}

#[test]
fn reported_artifact_failures_allow_the_caller_to_roll_back_publication() {
    let home = TempDir::new();
    let runtime = home.write(
        "lib/rt.jar",
        jar_bytes(&[("Runtime.class", &class_bytes("Runtime"))]),
    );
    let tools = home.write("lib/tools.jar", b"PK\x03\x04");
    let platform = JvmPlatform::new();
    let processed = platform.process(&runtime).unwrap();
    assert!(processed.failures.is_empty());
    let mut connection = database();
    let transaction = connection.transaction().unwrap();
    platform
        .store(&transaction, &runtime, processed.classes)
        .unwrap();

    let invalid = platform.process(&tools).unwrap();
    assert!(invalid.classes.is_empty());
    assert_eq!(invalid.failures.len(), 1);
    assert_eq!(invalid.failures[0].0, Location::file(&tools));
    assert!(!invalid.failures[0].1.is_empty());
    assert_eq!(names(&transaction), ["Runtime"]);
    transaction.rollback().unwrap();

    assert!(names(&connection).is_empty());
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
