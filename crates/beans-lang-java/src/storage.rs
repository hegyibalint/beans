use beans_core::location::{Location, LocationEntry};
use rusqlite::{Transaction, params};

use crate::{
    model::{File, nodes::NodeKind},
    query::entry_id,
};

pub const SCHEMA: &str = include_str!("storage/schema.sql");

/// Replaces the current model; the caller must roll back the transaction on error.
pub fn store(
    transaction: &Transaction<'_>,
    file: &File,
    location: &Location,
    contents: &str,
) -> Result<i64, String> {
    let model = serde_json::to_string(file).map_err(|error| error.to_string())?;
    let id = if let Some(id) = entry_id(transaction, location)? {
        transaction
            .execute(
                "UPDATE entries SET model = ?1, content = ?2 WHERE id = ?3",
                params![model, contents.as_bytes(), id],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM java_types WHERE entry_id = ?1", [id])
            .map_err(|error| error.to_string())?;
        id
    } else {
        insert(transaction, &model, location, contents)?
    };
    let mut insert = transaction
        .prepare("INSERT INTO java_types (entry_id, node_index, name) VALUES (?1, ?2, ?3)")
        .map_err(|error| error.to_string())?;
    for entry in file.iter_nodes() {
        if !matches!(entry.node.kind(), NodeKind::Type(_)) {
            continue;
        }
        let Some(name) = canonical_name(file, entry.index) else {
            continue;
        };
        let index =
            i64::try_from(entry.index.as_usize()).map_err(|_| "Java node index overflow")?;
        insert
            .execute(params![id, index, name])
            .map_err(|error| error.to_string())?;
    }
    Ok(id)
}

pub fn remove(transaction: &Transaction<'_>, location: &Location) -> Result<bool, String> {
    let Some(id) = entry_id(transaction, location)? else {
        return Ok(false);
    };
    transaction
        .execute("DELETE FROM entries WHERE id = ?1", [id])
        .map(|count| count != 0)
        .map_err(|error| error.to_string())
}

pub(crate) fn components(location: &Location) -> Result<Vec<(&str, &str)>, String> {
    location
        .entries
        .iter()
        .map(|entry| {
            Ok(match entry {
                LocationEntry::File(path) => {
                    ("file", path.to_str().ok_or("file location is not UTF-8")?)
                }
                LocationEntry::Virtual(value) => ("virtual", value.as_str()),
                LocationEntry::Container(value) => ("container", value.as_str()),
            })
        })
        .collect()
}

fn insert(
    transaction: &Transaction<'_>,
    model: &str,
    location: &Location,
    contents: &str,
) -> Result<i64, String> {
    let components = components(location)?;
    transaction
        .execute(
            "INSERT INTO entries (model_kind, model, content) VALUES (1, ?1, ?2)",
            params![model, contents.as_bytes()],
        )
        .map_err(|error| error.to_string())?;
    let id = transaction.last_insert_rowid();
    let mut insert = transaction
        .prepare("INSERT INTO locations (entry_id, position, kind, value) VALUES (?1, ?2, ?3, ?4)")
        .map_err(|error| error.to_string())?;
    for (position, (kind, value)) in components.into_iter().enumerate() {
        let position = i64::try_from(position).map_err(|_| "location position overflow")?;
        insert
            .execute(params![id, position, kind, value])
            .map_err(|error| error.to_string())?;
    }
    Ok(id)
}

fn canonical_name(file: &File, index: crate::model::nodes::NodeIndex) -> Option<String> {
    if !file.package_name.is_valid() {
        return None;
    }
    let mut types = Vec::new();
    for entry in file.iter_ancestors(index) {
        match entry.node.kind() {
            NodeKind::CompilationUnit => {}
            NodeKind::Type(declaration) => {
                let name = declaration.name()?;
                if name.is_empty() {
                    return None;
                }
                types.push(name);
            }
            _ => return None,
        }
    }
    Some(
        file.package_name
            .as_slice()
            .iter()
            .map(String::as_str)
            .chain(types.into_iter().rev())
            .collect::<Vec<_>>()
            .join("."),
    )
}
