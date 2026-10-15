use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use beans_core::{resource::ResourceId, revision::Revision};
use beans_lang_java::model::File;
use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};

mod model_query;
pub use model_query::ModelQuery;

pub struct Storage {
    writer: Connection,
    pub(crate) path: PathBuf,
}

impl Storage {
    pub fn new(path: &Path) -> Result<Self, String> {
        let writer = open(path)?;
        let path = std::path::absolute(path).map_err(|error| error.to_string())?;
        Ok(Self { writer, path })
    }

    pub(crate) fn reader(&self) -> Result<Connection, String> {
        let connection = Connection::open_with_flags(&self.path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| error.to_string())?;
        configure(&connection)?;
        Ok(connection)
    }

    pub(crate) fn publish_java(
        &mut self,
        source: &ResourceId,
        file: &File,
        contents: &str,
    ) -> Result<Revision, String> {
        let url = source.to_url()?;
        let model = beans_lang_java::codec::encode(file)?;
        let (transaction, revision) = begin_publication(&mut self.writer)?;
        transaction.execute(
            "INSERT INTO entries (resource_url, revision, model_kind, model, content) VALUES (?1, ?2, 1, ?3, ?4)",
            rusqlite::params![url, revision.as_i64(), model, contents.as_bytes()],
        ).map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(revision)
    }

    pub(crate) fn publish_jvm(
        &mut self,
        models: &[(String, Vec<u8>)],
    ) -> Result<Option<Revision>, String> {
        if models.is_empty() {
            return Ok(None);
        }
        let (transaction, revision) = begin_publication(&mut self.writer)?;
        {
            let mut insert = transaction.prepare(
                "INSERT INTO entries (resource_url, revision, model_kind, model) VALUES (?1, ?2, 0, ?3)",
            ).map_err(|error| error.to_string())?;
            for (url, model) in models {
                insert
                    .execute(rusqlite::params![url, revision.as_i64(), model])
                    .map_err(|error| error.to_string())?;
            }
        }
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(Some(revision))
    }
}

const SCHEMA_VERSION: i32 = 1;

/// Opens a file-backed index without choosing its location or creating parent directories.
pub fn open(path: &Path) -> Result<Connection, String> {
    let mut connection =
        Connection::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    configure(&connection)?;
    initialize(&mut connection)?;

    let mode: String = connection
        .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if mode != "wal" {
        return Err(format!("expected SQLite WAL mode, got {mode}"));
    }
    Ok(connection)
}

fn begin_publication(connection: &mut Connection) -> Result<(Transaction<'_>, Revision), String> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let latest: i64 = transaction
        .query_row(
            "SELECT COALESCE(MAX(revision), 0) FROM entries",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if latest < 0 {
        return Err("negative database revision".into());
    }
    let next = latest.checked_add(1).ok_or("revision overflow")?;
    Ok((transaction, Revision::new(next)))
}

fn configure(connection: &Connection) -> Result<(), String> {
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| error.to_string())
}

fn initialize(connection: &mut Connection) -> Result<(), String> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let version: i32 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    match version {
        0 => {
            let empty: bool = transaction
                .query_row(
                    "SELECT NOT EXISTS (SELECT 1 FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*')",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if !empty {
                return Err("cannot initialize a nonempty unversioned SQLite database".into());
            }
            transaction
                .execute_batch(include_str!("storage/schema.sql"))
                .map_err(|error| error.to_string())?;
            transaction
                .pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(|error| error.to_string())?;
        }
        SCHEMA_VERSION => {
            transaction
                .prepare("SELECT resource_url, revision, model_kind, model, content FROM entries LIMIT 0")
                .map_err(|error| error.to_string())?;
        }
        _ => return Err(format!("unsupported SQLite schema version: {version}")),
    }
    transaction.commit().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
