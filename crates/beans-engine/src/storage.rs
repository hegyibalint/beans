use std::{path::Path, time::Duration};

use rusqlite::{Connection, TransactionBehavior};

const SCHEMA_VERSION: i64 = 3;

/// Opens a file-backed cache. The caller chooses its location and creates its parent directory.
pub fn open(path: &Path) -> Result<Connection, String> {
    let mut connection =
        Connection::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    initialize(&mut connection, include_str!("storage/schema.sql"))?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| error.to_string())?;
    let mode: String = connection
        .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if mode != "wal" {
        return Err(format!("expected SQLite WAL mode, got {mode}"));
    }
    Ok(connection)
}

fn initialize(connection: &mut Connection, schema: &str) -> Result<(), String> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let version: i64 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if version > SCHEMA_VERSION {
        return Err(format!("unsupported SQLite schema version: {version}"));
    }
    if version < SCHEMA_VERSION {
        // Reset in place so existing WAL connections never refer to an unlinked database.
        let objects = {
            let mut statement = transaction
                .prepare(
                    "SELECT type, name FROM sqlite_schema
                     WHERE type IN ('view', 'table') AND name NOT GLOB 'sqlite_*'
                     ORDER BY type DESC",
                )
                .map_err(|error| error.to_string())?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| error.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?
        };
        for (kind, name) in objects {
            let name = name.replace('"', "\"\"");
            transaction
                .execute_batch(&format!("DROP {kind} \"{name}\""))
                .map_err(|error| error.to_string())?;
        }
        transaction
            .execute_batch(schema)
            .map_err(|error| error.to_string())?;
        transaction
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
