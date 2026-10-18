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
mod tests;
