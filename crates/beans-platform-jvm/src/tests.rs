use std::io::{Cursor, Write};

use beans_core::resource::{ResourceEntry, ResourceId, ResourceRoot};
use beans_testing::support::class_bytes;
use zip::{ZipWriter, write::SimpleFileOptions};

use super::*;

#[test]
fn produces_owned_classes_from_each_container_format() {
    let class = class_bytes("example/Foo");
    let source = ResourceId::new(ResourceRoot::Virtual {
        provider: "test".into(),
        key: "input".into(),
    })
    .entry(ResourceEntry("nested".into()));
    for (bytes, entry) in [
        (class.clone(), None),
        (jar_bytes(Some(&class)), Some("example/Foo.class")),
        (jimage_bytes(&class), Some("/java.base/example/Foo.class")),
    ] {
        let classes = read_classes(&source, &bytes).unwrap().unwrap();
        let expected = match entry {
            Some(entry) => source.entry(ResourceEntry(entry.into())),
            None => source.clone(),
        };
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].0, expected);
        assert_eq!(classes[0].1.name, "example/Foo");
    }
}

#[test]
fn recognized_empty_container_is_still_claimed() {
    let classes = read_classes(&ResourceId::file("input"), &jar_bytes(None))
        .unwrap()
        .unwrap();
    assert!(classes.is_empty());
}

#[test]
fn unknown_inputs_restore_the_readers_starting_position() {
    for bytes in [
        b"".as_slice(),
        b"x",
        b"not a container",
        b"\xCA\xFE\xBA",
        b"PK\x03",
    ] {
        let mut input = vec![0xAA; 7];
        input.extend(bytes);
        let mut reader = Cursor::new(input);
        reader.set_position(7);
        assert!(
            process(&ResourceId::file("input"), &mut reader)
                .unwrap()
                .is_none()
        );
        assert_eq!(reader.position(), 7);
    }
}

#[test]
fn recognized_errors_are_not_format_mismatches() {
    let source = ResourceId::file("input");
    for bytes in [
        b"\xCA\xFE\xBA\xBE".as_slice(),
        b"PK\x03\x04",
        b"\xDA\xDA\xFE\xCA",
    ] {
        let error = read_classes(&source, bytes).err().unwrap();
        assert!(error.contains("input"), "{error}");
    }
}

#[test]
fn class_decoding_is_lazy() {
    use std::{
        cell::Cell,
        io::{Read, Seek, SeekFrom},
        rc::Rc,
    };

    struct Reader {
        bytes: Cursor<Vec<u8>>,
        read: Rc<Cell<usize>>,
    }

    impl Read for Reader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let count = self.bytes.read(buffer)?;
            self.read.set(self.read.get() + count);
            Ok(count)
        }
    }

    impl Seek for Reader {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.bytes.seek(position)
        }
    }

    let bytes = class_bytes("example/Foo");
    let length = bytes.len();
    let read = Rc::new(Cell::new(0));
    let source = ResourceId::file("input");
    let mut classes = process(
        &source,
        Reader {
            bytes: Cursor::new(bytes),
            read: read.clone(),
        },
    )
    .unwrap()
    .unwrap();

    assert!(read.get() < length);
    assert_eq!(classes.next().unwrap().unwrap().1.name, "example/Foo");
    assert!(read.get() >= length);
    assert!(classes.next().is_none());
}

#[test]
fn reads_artifacts_from_each_jdk_runtime_layout() {
    let class = class_bytes("example/Foo");
    for (relative, bytes, entry) in [
        (
            "lib/modules",
            jimage_bytes(&class),
            "/java.base/example/Foo.class",
        ),
        (
            "jre/lib/rt.jar",
            jar_bytes(Some(&class)),
            "example/Foo.class",
        ),
        ("lib/rt.jar", jar_bytes(Some(&class)), "example/Foo.class"),
    ] {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();

        assert_eq!(jdk_artifacts(home.path()).unwrap(), vec![path.clone()]);
        let classes = read_artifacts(home.path()).unwrap();
        assert_eq!(classes.len(), 1);
        assert_eq!(
            classes[0].0,
            ResourceId::file(path).entry(ResourceEntry(entry.into()))
        );
        assert_eq!(classes[0].1.name, "example/Foo");
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

    let error = read_artifacts(home.path()).err().unwrap();
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
fn legacy_layouts_include_optional_tools() {
    for relative in ["jre/lib/rt.jar", "lib/rt.jar"] {
        let home = tempfile::tempdir().unwrap();
        let runtime = home.path().join(relative);
        let tools = home.path().join("lib/tools.jar");
        std::fs::create_dir_all(runtime.parent().unwrap()).unwrap();
        std::fs::create_dir_all(tools.parent().unwrap()).unwrap();
        std::fs::write(&runtime, jar_bytes(Some(&class_bytes("example/Runtime")))).unwrap();
        std::fs::write(&tools, jar_bytes(Some(&class_bytes("example/Tools")))).unwrap();

        assert_eq!(
            jdk_artifacts(home.path()).unwrap(),
            vec![runtime.clone(), tools.clone()]
        );
        let classes = read_artifacts(home.path()).unwrap();
        assert_eq!(classes.len(), 2);
        for (artifact, name) in [(runtime, "example/Runtime"), (tools, "example/Tools")] {
            let source =
                ResourceId::file(artifact).entry(ResourceEntry("example/Foo.class".into()));
            assert!(
                classes
                    .iter()
                    .any(|(resource, class)| resource == &source && class.name == name)
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

    assert_eq!(jdk_artifacts(home.path()).unwrap(), vec![runtime]);
    assert_eq!(read_artifacts(home.path()).unwrap().len(), 1);
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

    let error = read_artifacts(home.path()).err().unwrap();
    assert!(error.contains("lib/tools.jar"));
    assert!(error.contains("unrecognized container format"));
}

fn read_classes(
    source: &ResourceId,
    bytes: &[u8],
) -> Result<Option<Vec<(ResourceId, Class)>>, String> {
    process(source, Cursor::new(bytes))?
        .map(|classes| classes.collect())
        .transpose()
}

fn read_artifacts(home: &Path) -> Result<Vec<(ResourceId, Class)>, String> {
    let mut classes = Vec::new();
    for artifact in jdk_artifacts(home)? {
        let source = ResourceId::file(&artifact);
        let reader =
            std::fs::File::open(&artifact).map_err(|error| format!("{source:?}: {error}"))?;
        let entries = process(&source, reader)?
            .ok_or_else(|| format!("{source:?}: unrecognized container format"))?;
        for entry in entries {
            classes.push(entry?);
        }
    }
    Ok(classes)
}

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
