use beans_core::{
    location::{Location, LocationEntry},
    revision::Revision,
};
use rusqlite::{Transaction, params};

use crate::model::File;

/// Stores one model version; the caller must roll back the transaction on error.
pub fn store(
    transaction: &Transaction<'_>,
    revision: Revision,
    file: &File,
    location: &Location,
    contents: &str,
) -> Result<i64, String> {
    let model = serde_json::to_string(file).map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO entries (revision, model_kind, model, content) VALUES (?1, 1, ?2, ?3)",
            params![revision.as_i64(), model, contents.as_bytes()],
        )
        .map_err(|error| error.to_string())?;
    let id = transaction.last_insert_rowid();
    let mut insert = transaction
        .prepare("INSERT INTO locations (entry_id, position, kind, value) VALUES (?1, ?2, ?3, ?4)")
        .map_err(|error| error.to_string())?;
    for (position, entry) in location.entries.iter().enumerate() {
        let position = i64::try_from(position).map_err(|_| "location position overflow")?;
        let (kind, value) = match entry {
            LocationEntry::File(path) => {
                ("file", path.to_str().ok_or("file location is not UTF-8")?)
            }
            LocationEntry::Virtual(value) => ("virtual", value.as_str()),
            LocationEntry::Container(value) => ("container", value.as_str()),
        };
        insert
            .execute(params![id, position, kind, value])
            .map_err(|error| error.to_string())?;
    }
    Ok(id)
}
