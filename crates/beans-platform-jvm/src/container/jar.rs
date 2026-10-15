use std::io::{Read, Seek};

use beans_core::location::{Location, LocationEntry};
use zip::ZipArchive;

use crate::model::class::Class;

pub fn process(
    id: &Location,
    mut reader: impl Read + Seek,
) -> Result<Option<impl Iterator<Item = Result<(Location, Class), String>>>, String> {
    // Local file header, empty archive, or ZIP64 end record.
    if !matches!(
        super::magic(&mut reader).map_err(|error| format!("{id:?}: {error}"))?,
        Some([0x50, 0x4B, 0x03, 0x04] | [0x50, 0x4B, 0x05, 0x06] | [0x50, 0x4B, 0x06, 0x06])
    ) {
        return Ok(None);
    }

    let mut archive = ZipArchive::new(reader).map_err(|error| format!("{id:?}: {error}"))?;

    Ok(Some((0..archive.len()).filter_map(move |index| {
        let entry = match archive.by_index(index) {
            Ok(entry) => entry,
            Err(error) => {
                return Some(Err(format!("{id:?}: entry {index}: {error}")));
            }
        };

        if entry.is_dir() || entry.name().starts_with("META-INF/") {
            return None;
        }

        let entry_id = id.entry(LocationEntry::Container(entry.name().to_owned()));
        crate::parser::parse(entry)
            .map_err(|error| format!("{entry_id:?}: {error}"))
            .map(|class| class.map(|class| (entry_id, class)))
            .transpose()
    })))
}

#[cfg(test)]
mod tests;
