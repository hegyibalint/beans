use std::io::Read;

use beans_core::resource::ResourceId;

use crate::model::class::Class;

pub fn process(id: ResourceId, reader: impl Read) -> Result<Option<(ResourceId, Class)>, String> {
    let class = crate::parser::parse(reader).map_err(|error| format!("{id:?}: {error}"))?;
    Ok(class.map(|class| (id, class)))
}

#[cfg(test)]
mod tests;
