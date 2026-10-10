use std::io::{Read, Seek};

use beans_core::resource::ResourceId;

use crate::model::class::Class;

pub fn process(
    id: &ResourceId,
    mut reader: impl Read + Seek,
) -> Result<Option<impl Iterator<Item = Result<(ResourceId, Class), String>>>, String> {
    if super::magic(&mut reader).map_err(|error| format!("{id:?}: {error}"))?
        != Some([0xCA, 0xFE, 0xBA, 0xBE])
    {
        return Ok(None);
    }

    Ok(Some(std::iter::once_with(move || {
        let class = crate::parser::parse(reader)
            .map_err(|error| format!("{id:?}: {error}"))?
            .ok_or_else(|| format!("{id:?}: invalid class-file magic"))?;
        Ok((id.clone(), class))
    })))
}

#[cfg(test)]
mod tests;
