use std::io::{Read, Seek};

use beans_core::location::{Location, LocationEntry};
use zip::ZipArchive;

use crate::ProcessedResource;

use super::Entry;

pub fn process(
    id: &Location,
    mut reader: impl Read + Seek,
) -> Result<Option<ProcessedResource>, String> {
    // Local file header, empty archive, or ZIP64 end record.
    if !matches!(
        super::magic(&mut reader).map_err(|error| format!("{id:?}: {error}"))?,
        Some([0x50, 0x4B, 0x03, 0x04] | [0x50, 0x4B, 0x05, 0x06] | [0x50, 0x4B, 0x06, 0x06])
    ) {
        return Ok(None);
    }

    Ok(Some(super::collect(id, entries(id, reader))))
}

fn entries(id: &Location, reader: impl Read + Seek) -> Result<impl Iterator<Item = Entry>, String> {
    let mut archive = ZipArchive::new(reader).map_err(|error| error.to_string())?;

    Ok((0..archive.len()).filter_map(move |index| {
        let entry = match archive.by_index(index) {
            Ok(entry) => entry,
            Err(error) => {
                return Some(Err((id.clone(), format!("entry {index}: {error}"))));
            }
        };

        if entry.is_dir() || entry.name().starts_with("META-INF/") {
            return None;
        }

        let entry_id = id.entry(LocationEntry::Container(entry.name().to_owned()));
        let is_class = entry.name().ends_with(".class");
        match crate::parser::parse(entry) {
            Ok(Some(class)) => Some(Ok((entry_id, class))),
            Ok(None) if is_class => Some(Err((entry_id, "invalid class magic".into()))),
            Ok(None) => None,
            Err(error) => Some(Err((entry_id, error))),
        }
    }))
}
