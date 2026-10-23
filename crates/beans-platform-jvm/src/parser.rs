use std::io::Read;

use beans_core::location::Location;

use crate::model::class::Class;

pub fn parse(id: &Location, mut reader: impl Read) -> Result<Option<Class>, String> {
    let mut options = cafebabe::ParseOptions::default();
    options.parse_bytecode(false);

    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;

    // See JVMS §4: the class-file magic is 0xCAFEBABE.
    if !bytes.starts_with(&[0xCA, 0xFE, 0xBA, 0xBE]) {
        return Ok(None);
    }

    match cafebabe::parse_class_with_options(&bytes, &options) {
        Ok(class) => Ok(Some(Class {
            name: class.this_class.to_string(),
        })),
        Err(error) => {
            log::warn!("{id:?}: {error}");
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use beans_core::location::Location;

    use super::parse;

    #[test]
    fn unrecognized_contents_return_none() {
        for bytes in [b"".as_slice(), b"not a class", &[0xCA, 0xFE, 0xBA]] {
            assert!(parse(&Location::file("input"), bytes).unwrap().is_none());
        }
    }

    #[test]
    fn reader_failures_return_an_error() {
        struct BrokenReader;

        impl Read for BrokenReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("read failed"))
            }
        }

        assert!(
            matches!(parse(&Location::file("input"), BrokenReader), Err(error) if error == "read failed")
        );
    }
}
