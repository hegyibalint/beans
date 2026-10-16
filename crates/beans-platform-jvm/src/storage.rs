use std::path::Path;

use beans_core::location::{Location, LocationEntry};
use rusqlite::{Transaction, params};

use crate::Class;

/// Replaces the current model; the caller must roll back the transaction on error.
pub fn store(
    transaction: &Transaction<'_>,
    class: &Class,
    location: &Location,
) -> Result<i64, String> {
    let location = serde_json::to_string(location).map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM models WHERE model_kind = 0 AND location = ?1",
            [&location],
        )
        .map_err(|error| error.to_string())?;
    let model = serde_json::to_string(class).map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO models (location, model_kind, model) VALUES (?1, 0, ?2)",
            params![location, model],
        )
        .map_err(|error| error.to_string())?;
    Ok(transaction.last_insert_rowid())
}

/// Removes JVM models whose first location entry is the supplied physical resource.
pub fn remove(transaction: &Transaction<'_>, path: &Path) -> Result<bool, String> {
    let root = serde_json::to_string(&LocationEntry::File(path.into()))
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM models
             WHERE model_kind = 0 AND json_extract(location, '$[0]') = ?1",
            [root],
        )
        .map(|count| count != 0)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
