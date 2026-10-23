use std::io::{Read, Seek};

use beans_core::location::Location;

use crate::model::class::Class;

pub fn process(
    id: &Location,
    mut reader: impl Read + Seek,
) -> Result<Option<impl Iterator<Item = Result<(Location, Class), String>>>, String> {
    if super::magic(&mut reader).map_err(|error| format!("{id:?}: {error}"))?
        != Some([0xCA, 0xFE, 0xBA, 0xBE])
    {
        return Ok(None);
    }

    Ok(Some(
        std::iter::once_with(move || {
            crate::parser::parse(id, reader)
                .map_err(|error| format!("{id:?}: {error}"))
                .map(|class| class.map(|class| (id.clone(), class)))
                .transpose()
        })
        .flatten(),
    ))
}
