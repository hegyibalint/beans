//! Readers decline unrecognized formats without advancing the input.
//! Recognized formats return a result, including any invalid data; errors stop the chain.

pub mod class;
pub mod jar;
pub mod jimage;

use std::io::{self, Read, Seek, SeekFrom};

use beans_core::location::Location;

use crate::{Class, ProcessedResource};

type Entry = Result<(Location, Class), (Location, String)>;

fn collect(
    id: &Location,
    entries: Result<impl Iterator<Item = Entry>, String>,
) -> ProcessedResource {
    let mut processed = ProcessedResource::default();
    let entries = match entries {
        Ok(entries) => entries,
        Err(error) => {
            processed.failures.push((id.clone(), error));
            return processed;
        }
    };
    for entry in entries {
        match entry {
            Ok(class) => {
                processed.classes.push(class);
                if processed.classes.len() % 1000 == 0 {
                    log::debug!("Parsed {} JVM classes from {id:?}", processed.classes.len());
                }
            }
            Err(failure) => processed.failures.push(failure),
        }
    }
    log::debug!(
        "Finished parsing {} JVM classes from {id:?} with {} failures",
        processed.classes.len(),
        processed.failures.len()
    );
    processed
}

fn magic(reader: &mut (impl Read + Seek)) -> io::Result<Option<[u8; 4]>> {
    let start = reader.stream_position()?;
    let mut bytes = [0; 4];
    let result = reader.read_exact(&mut bytes);
    reader.seek(SeekFrom::Start(start))?;
    match result {
        Ok(()) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn readers_decline_unknown_inputs_without_advancing_the_reader() {
        let id = Location::file("input");
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

            assert!(class::process(&id, &mut reader).unwrap().is_none());
            assert_eq!(reader.position(), 7);
            assert!(jar::process(&id, &mut reader).unwrap().is_none());
            assert_eq!(reader.position(), 7);
            assert!(jimage::process(&id, &mut reader).unwrap().is_none());
            assert_eq!(reader.position(), 7);
        }
    }

    #[test]
    fn readers_decline_other_container_formats() {
        let id = Location::file("input");
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

        let mut reader = BrokenReader(Cursor::new(vec![0; 8]));
        reader.0.set_position(7);

        let id = Location::file("input");
        for error in [
            class::process(&id, &mut reader).unwrap_err(),
            jar::process(&id, &mut reader).unwrap_err(),
            jimage::process(&id, &mut reader).unwrap_err(),
        ] {
            assert!(error.contains("read failed"));
        }
        assert_eq!(reader.0.position(), 7);
    }
}
