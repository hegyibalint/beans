use std::io::{self, Cursor, Write};

use beans_core::resource::{ResourceEntry, ResourceId, ResourceRoot};
use beans_testing::support::class_bytes;
use zip::{ZipWriter, write::SimpleFileOptions};

use super::{Class, process};

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

fn read_classes(
    source: &ResourceId,
    bytes: &[u8],
) -> Result<Option<Vec<(ResourceId, Class)>>, String> {
    process(source, Cursor::new(bytes))?
        .map(|classes| classes.collect())
        .transpose()
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
