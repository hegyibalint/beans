use std::io::Cursor;

use super::*;
use crate::container::jimage::tests::{Entry, Image, word};

fn read(bytes: &[u8]) -> Result<Index, FormatError> {
    Index::read(&mut Cursor::new(bytes))
}

fn entry(index: &Index, slot: usize) -> Location {
    let offset = index.offset(slot).expect("the image has this entry");
    index.location(offset).expect("its stream should decode")
}

#[test]
fn an_image_is_read_whichever_way_round_it_was_written() {
    for order in [Order::Little, Order::Big] {
        let mut image = Image::new();
        image.add(Entry {
            module: "java.base",
            parent: "java/lang",
            base: "String",
            extension: "class",
        });

        let index = read(&image.write(order)).expect("the image should read");

        assert_eq!(index.order(), order);
        let location = entry(&index, 0);
        assert_eq!(index.module(&location).unwrap(), "java.base");
        assert_eq!(
            index.resource_path(&location).unwrap(),
            "java/lang/String.class"
        );
    }
}

#[test]
fn a_name_leaves_out_the_parts_the_entry_has_none_of() {
    let mut image = Image::new();
    image.add(Entry {
        module: "java.base",
        base: "module-info",
        extension: "class",
        ..Entry::default()
    });
    image.add(Entry {
        base: "packages",
        ..Entry::default()
    });

    let index = read(&image.write(Order::Little)).expect("the image should read");

    assert_eq!(
        index.resource_path(&entry(&index, 0)).unwrap(),
        "module-info.class"
    );

    // A root of the `jrt:` view carries no module at all. Offset zero is the
    // empty string rather than a missing one, so it needs no special case.
    let root = entry(&index, 1);
    assert_eq!(index.module(&root).unwrap(), "");
    assert_eq!(index.resource_path(&root).unwrap(), "packages");
}

#[test]
fn an_attribute_kind_we_have_never_heard_of_is_skipped() {
    let mut image = Image::new();
    let module = image.string("java.base");
    let base = image.string("String");
    let offset = image.location(&[
        (MODULE, u64::from(module)),
        (12, 0xDEAD),
        (BASE, u64::from(base)),
    ]);
    image.index(offset);

    let index = read(&image.write(Order::Little)).expect("the image should read");

    let location = entry(&index, 0);
    assert_eq!(index.module(&location).unwrap(), "java.base");
    assert_eq!(index.resource_path(&location).unwrap(), "String");
}

#[test]
fn a_character_above_the_bmp_survives_its_surrogate_pair() {
    let mut image = Image::new();
    // U+1F600, which modified UTF-8 writes as the two UTF-16 surrogates
    // encoded one at a time. Real UTF-8 would give it four bytes, and
    // `str::from_utf8` refuses these six.
    let base = image.raw_string(&[0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80]);
    let offset = image.location(&[(BASE, u64::from(base))]);
    image.index(offset);

    let index = read(&image.write(Order::Little)).expect("the image should read");

    assert_eq!(index.resource_path(&entry(&index, 0)).unwrap(), "\u{1F600}");
}

#[test]
fn a_nul_character_is_two_bytes_and_does_not_end_the_string() {
    let mut image = Image::new();
    let base = image.raw_string(&[b'A', 0xC0, 0x80, b'B']);
    let offset = image.location(&[(BASE, u64::from(base))]);
    image.index(offset);

    let index = read(&image.write(Order::Little)).expect("the image should read");

    assert_eq!(index.resource_path(&entry(&index, 0)).unwrap(), "A\0B");
}

#[test]
fn bytes_that_are_not_an_image_are_refused() {
    assert!(matches!(read(b"PK\x03\x04"), Err(FormatError::NotAnImage)));
    assert!(matches!(read(b""), Err(FormatError::Truncated)));
}

#[test]
fn an_image_of_another_major_version_is_refused() {
    let mut image = Image::new();
    image.add(Entry {
        base: "String",
        ..Entry::default()
    });
    let mut bytes = image.write(Order::Little);
    bytes[4..8].copy_from_slice(&word(Order::Little, 2 << 16 | 3));

    assert!(matches!(
        read(&bytes),
        Err(FormatError::Version { major: 2, minor: 3 })
    ));
}

#[test]
fn an_index_shorter_than_the_header_claims_is_refused() {
    let mut image = Image::new();
    image.add(Entry {
        base: "String",
        ..Entry::default()
    });
    let mut bytes = image.write(Order::Little);
    bytes.truncate(bytes.len() - 1);

    assert!(matches!(read(&bytes), Err(FormatError::Truncated)));
}

#[test]
fn an_attribute_value_running_past_its_region_is_refused() {
    let mut image = Image::new();
    let offset = image.attributes_mut().len() as u32;
    // A header byte claiming eight value bytes, with two to follow.
    image.attributes_mut().push((BASE as u8) << 3 | 7);
    image.attributes_mut().extend_from_slice(&[0, 0]);
    image.index(offset);

    let index = read(&image.write(Order::Little)).expect("the header is sound");

    assert!(matches!(
        index.location(index.offset(0).unwrap()),
        Err(FormatError::Attribute)
    ));
}

#[test]
fn a_string_offset_the_table_does_not_reach_is_refused() {
    let mut image = Image::new();
    let offset = image.location(&[(BASE, 9999)]);
    image.index(offset);

    let index = read(&image.write(Order::Little)).expect("the header is sound");

    assert!(matches!(
        index.resource_path(&entry(&index, 0)),
        Err(FormatError::String)
    ));
}

#[test]
fn a_string_that_never_ends_is_refused() {
    let mut image = Image::new();
    let base = image.strings_mut().len() as u32;
    image.strings_mut().extend_from_slice(b"String");
    let offset = image.location(&[(BASE, u64::from(base))]);
    image.index(offset);

    let index = read(&image.write(Order::Little)).expect("the header is sound");

    assert!(matches!(
        index.resource_path(&entry(&index, 0)),
        Err(FormatError::String)
    ));
}
