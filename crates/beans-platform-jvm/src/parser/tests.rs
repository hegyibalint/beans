use std::io;

use super::*;
use crate::tests::support::class_bytes;

#[test]
fn returns_the_owned_class_name() {
    let class = {
        let bytes = class_bytes("example/Foo");
        parse(bytes.as_slice()).unwrap().unwrap()
    };

    assert_eq!(class.name, "example/Foo");
}

#[test]
fn unrecognized_contents_return_none() {
    for bytes in [b"".as_slice(), b"not a class", &[0xCA, 0xFE, 0xBA]] {
        assert!(parse(bytes).unwrap().is_none());
    }
}

#[test]
fn recognized_but_truncated_class_returns_an_error() {
    assert!(parse([0xCA, 0xFE, 0xBA, 0xBE].as_slice()).is_err());
}

#[test]
fn reader_failures_return_an_error() {
    struct BrokenReader;

    impl Read for BrokenReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("read failed"))
        }
    }

    assert!(matches!(parse(BrokenReader), Err(error) if error == "read failed"));
}
