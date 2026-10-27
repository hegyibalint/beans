use beans_core::location::Location;
use rusqlite::{Connection, OptionalExtension};

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
