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

    pub fn process(&mut self, source: Location, contents: &str) -> Result<bool, String> {
        let [LocationEntry::File(path)] = source.entries.as_slice() else {
            return Ok(false);
        };
        if !self.accept(path) {
            return Ok(false);
        }
        self.process_document(source, "java", contents)
    }

    pub fn process_document(
        &mut self,
        source: Location,
        language_id: &str,
        contents: &str,
    ) -> Result<bool, String> {
        if language_id != "java" {
            return Ok(false);
        }
        let model = self.java.process(contents);
        let transaction = self
            .storage
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        storage::store(&transaction, &model, &source, contents)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(true)
    }

    pub fn close_document(&mut self, source: Location) -> Result<bool, String> {
        let transaction = self
            .storage
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let changed = storage::remove(&transaction, &source)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(changed)
    }
}
