use beans_core::{location::Location, names::Name};
use rusqlite::{Connection, OptionalExtension};

use crate::{
    model::{
        File,
        nodes::{NodeIndex, types::TypeDeclaration},
    },
    semantics::DeclarationHandle,
};

pub fn file(location: &Location, connection: &Connection) -> Result<Option<File>, String> {
    let Some(id) = model_id(location, connection)? else {
        return Ok(None);
    };
    let model: String = connection
        .query_row("SELECT model FROM models WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())?;
    let file: File = serde_json::from_str(&model).map_err(|error| error.to_string())?;
    file.validate_nodes()?;
    Ok(Some(file))
}

pub fn declaration(
    handle: &DeclarationHandle,
    connection: &Connection,
) -> Result<Option<TypeDeclaration>, String> {
    Ok(file(handle.source(), connection)?.and_then(|file| {
        file.node(handle.node_index())
            .and_then(|node| node.kind().as_type())
            .cloned()
    }))
}

pub fn find_type(name: &Name, connection: &Connection) -> Result<Vec<DeclarationHandle>, String> {
    if !name.is_valid() || name.is_empty() {
        return Ok(Vec::new());
    }
    let sql = "SELECT model_id, node_index FROM java_types WHERE name = ?1
               ORDER BY model_id, node_index";
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([name.to_string()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| error.to_string())?;
    let mut handles = Vec::new();
    for row in rows {
        let (id, index) = row.map_err(|error| error.to_string())?;
        let source = location(id, connection)?;
        let index = usize::try_from(index).map_err(|_| "invalid Java node index")?;
        handles.push(DeclarationHandle::new(source, NodeIndex::new(index)));
    }
    Ok(handles)
}

pub(crate) fn model_id(
    location: &Location,
    connection: &Connection,
) -> Result<Option<i64>, String> {
    connection
        .query_row(
            "SELECT id FROM models WHERE kind = 1 AND location = ?1",
            [serde_json::to_string(location).map_err(|error| error.to_string())?],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn location(id: i64, connection: &Connection) -> Result<Location, String> {
    let location: String = connection
        .query_row("SELECT location FROM models WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&location).map_err(|error| error.to_string())
}
