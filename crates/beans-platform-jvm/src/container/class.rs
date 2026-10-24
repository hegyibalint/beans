use std::io::{Read, Seek};

use beans_core::location::Location;

use crate::ProcessedResource;

pub fn process(
    id: &Location,
    mut reader: impl Read + Seek,
) -> Result<Option<ProcessedResource>, String> {
    if super::magic(&mut reader).map_err(|error| format!("{id:?}: {error}"))?
        != Some([0xCA, 0xFE, 0xBA, 0xBE])
    {
        return Ok(None);
    }

    let entry = crate::parser::parse(reader)
        .and_then(|class| class.ok_or_else(|| "invalid class magic".into()))
        .map(|class| (id.clone(), class))
        .map_err(|error| (id.clone(), error));
    Ok(Some(super::collect(id, Ok(std::iter::once(entry)))))
}
