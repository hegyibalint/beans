use std::path::Path;

use beans_core::location::{Location, LocationEntry};
use rusqlite::{Transaction, params};

use crate::Class;

/// Inserts a model; the caller owns resource replacement and must roll back on error.
pub fn store(
    class: &Class,
    location: &Location,
    transaction: &Transaction<'_>,
) -> Result<i64, String> {
    let location = serde_json::to_string(location).map_err(|error| error.to_string())?;
    let model = serde_json::to_string(class).map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO models (location, kind, model) VALUES (?1, 0, ?2)",
            params![location, model],
        )
        .map_err(|error| error.to_string())?;
    Ok(transaction.last_insert_rowid())
}

/// Removes JVM models whose first location entry is the supplied physical resource.
pub fn remove(path: &Path, transaction: &Transaction<'_>) -> Result<bool, String> {
    let root = serde_json::to_string(&LocationEntry::File(path.into()))
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM models
             WHERE kind = 0 AND json_extract(location, '$[0]') = ?1",
            [root],
        )
        .map(|count| count != 0)
        .map_err(|error| error.to_string())
}
