use std::io::{Cursor, Write};

use beans_core::{classpath::Unrestricted, query_scope::QueryScope};
use beans_testing::support::class_bytes;
use tempfile::NamedTempFile;
use zip::{ZipWriter, write::SimpleFileOptions};

use super::*;

fn jar_bytes(class: Option<&[u8]>) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    if let Some(bytes) = class {
        writer
            .start_file("example/Foo.class", SimpleFileOptions::default())
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

#[test]
fn parses_each_container_format_without_retaining_classes() {
    let class = class_bytes("example/Foo");
    for bytes in [class.clone(), jar_bytes(Some(&class)), jimage_bytes(&class)] {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&bytes).unwrap();
        let mut platform = JvmPlatform::default();

        platform.process(file.path().to_owned()).unwrap();

        assert!(
            platform
                .find_classes(QueryScope::new(&Unrestricted), "example/Foo")
                .is_empty()
        );
    }
}

#[test]
fn recognized_empty_container_does_not_fall_through() {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&jar_bytes(None)).unwrap();
    let mut platform = JvmPlatform::new();

    platform.process(file.path().to_owned()).unwrap();
    assert!(
        platform
            .find_classes(QueryScope::new(&Unrestricted), "example/Foo")
            .is_empty()
    );
}

#[test]
fn reports_unknown_formats_and_preserves_recognized_errors() {
    for (bytes, recognized) in [
        (b"not a container".as_slice(), false),
        (&[0xCA, 0xFE, 0xBA, 0xBE], true),
        (b"PK\x03\x04", true),
        (&[0xDA, 0xDA, 0xFE, 0xCA], true),
    ] {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        let mut platform = JvmPlatform::default();
        let error = platform.process(file.path().to_owned()).unwrap_err();

        assert_eq!(error.contains("unrecognized container format"), !recognized);
    }
}

#[test]
fn imports_each_jdk_runtime_layout() {
    let class = class_bytes("example/Foo");
    for (relative, bytes) in [
        ("lib/modules", jimage_bytes(&class)),
        ("jre/lib/rt.jar", jar_bytes(Some(&class))),
        ("lib/rt.jar", jar_bytes(Some(&class))),
    ] {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        let mut platform = JvmPlatform::default();
        assert_eq!(jdk_artifacts(home.path()).unwrap(), vec![path.clone()]);
        platform.process_jdk_home(home.path()).unwrap();

        assert!(
            platform
                .find_classes(QueryScope::new(&Unrestricted), "example/Foo")
                .is_empty()
        );
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
    std::fs::write(home.path().join("jre/lib/rt.jar"), jar_bytes(None)).unwrap();
    let mut platform = JvmPlatform::default();

    let error = platform.process_jdk_home(home.path()).unwrap_err();

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
    std::fs::write(home.path().join("lib/rt.jar"), jar_bytes(None)).unwrap();
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
        std::fs::write(&runtime, jar_bytes(Some(&class_bytes("example/Runtime")))).unwrap();
        std::fs::write(&tools, jar_bytes(Some(&class_bytes("example/Tools")))).unwrap();
        let mut platform = JvmPlatform::default();
        assert_eq!(
            jdk_artifacts(home.path()).unwrap(),
            vec![runtime.clone(), tools.clone()]
        );
        platform.process_jdk_home(home.path()).unwrap();

        for name in ["example/Runtime", "example/Tools"] {
            assert!(
                platform
                    .find_classes(QueryScope::new(&Unrestricted), name)
                    .is_empty()
            );
        }
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
    let mut platform = JvmPlatform::default();
    assert_eq!(jdk_artifacts(home.path()).unwrap(), vec![runtime]);
    platform.process_jdk_home(home.path()).unwrap();
    assert!(
        platform
            .find_classes(QueryScope::new(&Unrestricted), "example/Foo")
            .is_empty()
    );
}

#[test]
fn tools_alone_do_not_supply_a_runtime() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("lib")).unwrap();
    std::fs::write(home.path().join("lib/tools.jar"), jar_bytes(None)).unwrap();

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
        std::fs::write(&runtime, jar_bytes(None)).unwrap();
        std::fs::create_dir_all(home.path().join("lib/tools.jar")).unwrap();

        let error = jdk_artifacts(home.path()).unwrap_err();
        assert!(error.contains("lib/tools.jar"));
        assert!(error.contains("expected a JDK artifact file"));
    }
}

#[test]
fn parsing_optional_artifact_errors_are_not_ignored() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("lib")).unwrap();
    std::fs::write(home.path().join("lib/rt.jar"), jar_bytes(None)).unwrap();
    std::fs::write(home.path().join("lib/tools.jar"), b"invalid tools").unwrap();
    let mut platform = JvmPlatform::default();

    let error = platform.process_jdk_home(home.path()).unwrap_err();
    assert!(error.contains("lib/tools.jar"));
    assert!(error.contains("unrecognized container format"));
}
