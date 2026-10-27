use beans_core::location::Location;
use rusqlite::{Transaction, params};

use crate::{
    model::{File, nodes::NodeKind},
    query::model_id,
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
    let id = if let Some(id) = model_id(transaction, location)? {
        transaction
            .execute(
                "UPDATE models SET model = ?1, content = ?2 WHERE id = ?3",
                params![model, contents.as_bytes(), id],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM java_types WHERE model_id = ?1", [id])
            .map_err(|error| error.to_string())?;
        id
    } else {
        let location = serde_json::to_string(location).map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO models (location, kind, model, content) VALUES (?1, 1, ?2, ?3)",
                params![location, model, contents.as_bytes()],
            )
            .map_err(|error| error.to_string())?;
        transaction.last_insert_rowid()
    };
    let mut insert = transaction
        .prepare("INSERT INTO java_types (model_id, node_index, name) VALUES (?1, ?2, ?3)")
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
    let Some(id) = model_id(transaction, location)? else {
        return Ok(false);
    };
    transaction
        .execute("DELETE FROM models WHERE id = ?1", [id])
        .map(|count| count != 0)
        .map_err(|error| error.to_string())
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
