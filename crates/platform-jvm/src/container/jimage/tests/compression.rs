use std::io::{Cursor, Write};
use std::path::Path;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use super::Image;
use crate::container::jimage::index::{Index, Order};
use crate::container::jimage::{COMPRESSED_MAGIC, expand};

/// An image holding one string, which is all `expand` reads an index for: a
/// compressed entry names its decompressor by an offset into the string table.
fn image_naming(decompressor: &str) -> (Index, u32) {
    let mut image = Image::new();
    let offset = image.string(decompressor);
    let index =
        Index::read(&mut Cursor::new(image.write(Order::Little))).expect("the fixture should read");
    (index, offset)
}

/// `CompressedResourceHeader` in front of zlib-deflated content, the way
/// `jlink --compress=zip-N` leaves it.
fn compressed(decompressor: u32, content: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content).expect("an in-memory write");
    let payload = encoder.finish().expect("an in-memory flush");

    let mut resource = Vec::new();
    resource.extend_from_slice(&COMPRESSED_MAGIC.to_le_bytes());
    resource.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    resource.extend_from_slice(&(content.len() as u64).to_le_bytes());
    resource.extend_from_slice(&decompressor.to_le_bytes());
    resource.extend_from_slice(&(-1i32).to_le_bytes());
    resource.push(1);
    resource.extend_from_slice(&payload);
    resource
}

fn class_file() -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/class_file/tests/fixtures/classes/beans/fixture/Point.class");
    std::fs::read(path).expect("the fixture should read")
}

#[test]
fn a_zip_entry_expands_to_the_bytes_that_went_in() {
    let content = class_file();
    let (index, decompressor) = image_naming("zip");
    let mut buffer = compressed(decompressor, &content);

    expand(&index, &mut buffer).expect("a zip entry should expand");

    assert_eq!(buffer, content);
}

#[test]
fn bytes_no_compressor_touched_are_left_alone() {
    let content = class_file();
    let (index, _) = image_naming("zip");
    let mut buffer = content.clone();

    expand(&index, &mut buffer).expect("uncompressed bytes should pass through");

    assert_eq!(buffer, content);
}

#[test]
fn a_compressor_we_do_not_implement_is_named() {
    // `--compress=1` still writes this one. It strips a class file's UTF-8
    // constant pool into the image string table, so undoing it is class-file
    // writing rather than decompression.
    let (index, decompressor) = image_naming("compact-cp");
    let mut buffer = compressed(decompressor, b"whatever");

    let error = expand(&index, &mut buffer).expect_err("it should not expand");

    assert!(error.to_string().contains("compact-cp"), "{error}");
}
