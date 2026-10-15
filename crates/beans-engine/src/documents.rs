use std::path::Path;

use beans_core::location::{Location, LocationEntry};
use beans_lang_java::storage;
use rusqlite::TransactionBehavior;

use crate::Engine;

#[cfg(test)]
mod tests;

impl Engine {
    pub fn accept(&self, path: &Path) -> bool {
        self.java.accept(path)
    }

    pub fn process(&mut self, source: Location, contents: &str) -> Result<(), String> {
        let [LocationEntry::File(path)] = source.entries.as_slice() else {
            return Ok(());
        };
        if !self.accept(path) {
            return Ok(());
        }
        self.process_document(source, "java", contents)
    }

    pub fn process_document(
        &mut self,
        source: Location,
        language_id: &str,
        contents: &str,
    ) -> Result<(), String> {
        if language_id != "java" {
            return Ok(());
        }
        let model = self.java.process(contents);
        let transaction = self
            .storage
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let revision = next_revision(&transaction)?;
        storage::store(&transaction, revision, &model, &source, contents)?;
        transaction.commit().map_err(|error| error.to_string())?;
        self.revision = revision;
        Ok(())
    }

    pub fn close_document(&mut self, source: Location) -> Result<(), String> {
        let transaction = self
            .storage
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let revision = next_revision(&transaction)?;
        storage::remove(&transaction, revision, &source)?;
        transaction.commit().map_err(|error| error.to_string())?;
        self.revision = revision;
        Ok(())
    }
}

fn next_revision(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<beans_core::revision::Revision, String> {
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
    Ok(beans_core::revision::Revision::new(next))
}
