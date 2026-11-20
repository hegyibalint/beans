use beans_core::location::Location;
use rusqlite::{Connection, OptionalExtension};

use crate::Class;

/// Loads a class by internal name without guessing between indexed copies.
pub fn unique_class_by_name(name: &str, connection: &Connection) -> Result<Option<Class>, String> {
    let mut statement = connection
        .prepare(
            "SELECT model FROM models
             WHERE kind = 0 AND json_extract(model, '$.name') = ?1
             LIMIT 2",
        )
        .map_err(|error| error.to_string())?;
    let mut rows = statement.query([name]).map_err(|error| error.to_string())?;
    let Some(row) = rows.next().map_err(|error| error.to_string())? else {
        return Ok(None);
    };
    let model: String = row.get(0).map_err(|error| error.to_string())?;
    if rows.next().map_err(|error| error.to_string())?.is_some() {
        return Err(format!("More than one indexed JVM class matches {name}"));
    }
    serde_json::from_str(&model)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub fn class_name(location: &Location, connection: &Connection) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT json_extract(model, '$.name') FROM models
             WHERE kind = 0 AND location = ?1",
            [serde_json::to_string(location).map_err(|error| error.to_string())?],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn find_classes(binary_name: &str, connection: &Connection) -> Result<Vec<Location>, String> {
    let mut statement = connection
        .prepare(
            "SELECT location FROM models
             WHERE kind = 0 AND json_extract(model, '$.name') = ?1
             ORDER BY id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([binary_name], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    let mut locations = Vec::new();
    for row in rows {
        let location = row.map_err(|error| error.to_string())?;
        let location: Location =
            serde_json::from_str(&location).map_err(|error| error.to_string())?;
        locations.push(location);
    }
    Ok(locations)
}
