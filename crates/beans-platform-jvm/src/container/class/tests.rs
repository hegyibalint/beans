use std::io::Cursor;

use super::*;
use beans_testing::support::class_bytes;

#[test]
fn yields_one_class() {
    let id = Location::file("Example.class");
    let mut entries = process(&id, Cursor::new(class_bytes("example/Foo")))
        .unwrap()
        .unwrap();
    let (entry_id, class) = entries.next().unwrap().unwrap();
    assert_eq!(entry_id, id);
    assert_eq!(class.name, "example/Foo");
    assert!(entries.next().is_none());
}

#[test]
fn recognized_but_truncated_class_yields_error() {
    let id = Location::file("broken.class");
    let mut entries = process(&id, Cursor::new([0xCA, 0xFE, 0xBA, 0xBE]))
        .unwrap()
        .unwrap();
    assert!(entries.next().unwrap().is_err());
    assert!(entries.next().is_none());
}
