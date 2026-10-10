use std::io::{Cursor, Write};

use zip::{ZipWriter, write::SimpleFileOptions};

use super::*;
use crate::tests::support::class_bytes;

fn archive(entries: &[(&str, &[u8])]) -> Cursor<Vec<u8>> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    let mut reader = writer.finish().unwrap();
    reader.set_position(0);
    reader
}

#[test]
fn recognizes_empty_and_resource_only_archives() {
    for reader in [archive(&[]), archive(&[("readme.txt", b"hello")])] {
        let id = ResourceId::file("empty.jar");
        let mut entries = process(&id, reader).unwrap().unwrap();
        assert!(entries.next().is_none());
    }
}

#[test]
fn yields_classes_and_skips_resources() {
    let bytes = class_bytes("example/Foo");
    let reader = archive(&[("readme.txt", b"hello"), ("example/Foo.class", &bytes)]);
    let id = ResourceId::file("library.jar");
    let mut entries = process(&id, reader).unwrap().unwrap();
    let (entry_id, class) = entries.next().unwrap().unwrap();
    assert_eq!(
        entry_id,
        id.entry(ResourceEntry("example/Foo.class".into()))
    );
    assert_eq!(class.name, "example/Foo");
    assert!(entries.next().is_none());
}

#[test]
fn rejects_recognized_but_truncated_zip() {
    for signature in [*b"PK\x03\x04", *b"PK\x05\x06", *b"PK\x06\x06"] {
        assert!(process(&ResourceId::file("broken.jar"), Cursor::new(signature)).is_err());
    }
}
