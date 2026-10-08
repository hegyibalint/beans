use std::io::Cursor;

use zip::{ZipWriter, write::SimpleFileOptions};

use super::*;

#[test]
fn empty_archive_yields_no_entries() {
    let archive = ZipWriter::new(Cursor::new(Vec::new())).finish().unwrap();
    let mut entries = process(ResourceId::file("empty.jar"), archive).unwrap();

    assert!(entries.next().is_none());
}

#[test]
fn directories_and_meta_inf_entries_are_not_sent_to_the_class_processor() {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    archive.add_directory("example/", options).unwrap();
    archive.start_file("META-INF/MANIFEST.MF", options).unwrap();
    archive
        .start_file("META-INF/versions/9/example/Foo.class", options)
        .unwrap();

    let mut entries = process(ResourceId::file("library.jar"), archive.finish().unwrap()).unwrap();

    assert!(entries.next().is_none());
}

#[test]
fn invalid_archive_returns_an_opening_error_with_its_identity() {
    let result = process(ResourceId::file("broken.jar"), Cursor::new(b"not a ZIP"));
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("invalid archive was accepted"),
    };

    assert!(error.contains("broken.jar"));
}
