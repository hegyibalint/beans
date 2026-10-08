use std::io::{Read, Seek};

use beans_core::resource::{ResourceEntry, ResourceId};
use zip::ZipArchive;

use crate::model::class::Class;

pub fn process(
    id: ResourceId,
    reader: impl Read + Seek,
) -> Result<impl Iterator<Item = Result<(ResourceId, Class), String>>, String> {
    let mut archive = ZipArchive::new(reader).map_err(|error| format!("{id:?}: {error}"))?;

    Ok((0..archive.len()).filter_map(move |index| {
        let entry = match archive.by_index(index) {
            Ok(entry) => entry,
            Err(error) => {
                return Some(Err(format!("{id:?}: entry {index}: {error}")));
            }
        };

        if entry.is_dir() || entry.name().starts_with("META-INF/") {
            return None;
        }

        let entry_id = id.entry(ResourceEntry(entry.name().to_owned()));
        super::class::process(entry_id, entry).transpose()
    }))
}

#[cfg(test)]
mod tests;
