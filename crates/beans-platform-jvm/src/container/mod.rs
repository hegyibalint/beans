//! Container readers return `Ok(None)` when the format is not recognized,
//! leaving the reader at its starting position so another reader can try.
//! `Ok(Some(entries))` claims the input, even if the iterator yields no classes.
//! Opening errors and errors yielded during iteration must not trigger fallback.

pub mod class;
pub mod jar;
pub mod jimage;

use std::io::{self, Read, Seek, SeekFrom};

fn magic(reader: &mut (impl Read + Seek)) -> Result<Option<[u8; 4]>, String> {
    let start = reader
        .stream_position()
        .map_err(|error| error.to_string())?;
    let mut bytes = [0; 4];
    let result = reader.read_exact(&mut bytes);
    reader
        .seek(SeekFrom::Start(start))
        .map_err(|error| error.to_string())?;
    match result {
        Ok(()) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use beans_core::resource::ResourceId;

    use super::*;

    #[test]
    fn unrecognized_inputs_restore_the_starting_position() {
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
                class::process(&ResourceId::file("input"), &mut reader)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(reader.position(), 7);
            assert!(
                jar::process(&ResourceId::file("input"), &mut reader)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(reader.position(), 7);
            assert!(
                jimage::process(&ResourceId::file("input"), &mut reader)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(reader.position(), 7);
        }
    }

    #[test]
    fn readers_decline_other_container_formats() {
        let id = ResourceId::file("input");
        let class_magic = [0xCA, 0xFE, 0xBA, 0xBE];
        let jar_magic = *b"PK\x03\x04";
        let image_magic = [0xDA, 0xDA, 0xFE, 0xCA];
        assert!(
            class::process(&id, Cursor::new(jar_magic))
                .unwrap()
                .is_none()
        );
        assert!(
            class::process(&id, Cursor::new(image_magic))
                .unwrap()
                .is_none()
        );
        assert!(
            jar::process(&id, Cursor::new(class_magic))
                .unwrap()
                .is_none()
        );
        assert!(
            jar::process(&id, Cursor::new(image_magic))
                .unwrap()
                .is_none()
        );
        assert!(
            jimage::process(&id, Cursor::new(class_magic))
                .unwrap()
                .is_none()
        );
        assert!(
            jimage::process(&id, Cursor::new(jar_magic))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn io_errors_are_not_format_mismatches() {
        struct BrokenReader(Cursor<Vec<u8>>);

        impl Read for BrokenReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("read failed"))
            }
        }
        impl Seek for BrokenReader {
            fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
                self.0.seek(position)
            }
        }

        let mut reader = BrokenReader(Cursor::new(Vec::new()));
        assert!(class::process(&ResourceId::file("input"), &mut reader).is_err());
        assert!(jar::process(&ResourceId::file("input"), &mut reader).is_err());
        assert!(jimage::process(&ResourceId::file("input"), &mut reader).is_err());
    }
}
