#![allow(dead_code)]

use beans_core::location::Location;
use beans_lang_java::{engine::JavaLanguage, model::File, storage};
use rusqlite::Connection;

pub struct Models {
    pub connection: Connection,
}

impl Default for Models {
    fn default() -> Self {
        Self::new()
    }
}

impl Models {
    pub fn new() -> Self {
        Self {
            connection: beans_testing::database(),
        }
    }

    pub fn process(&self, contents: &str) -> File {
        JavaLanguage::new().process(contents)
    }

    pub fn store(&mut self, file: File, source: Location) {
        let transaction = self.connection.transaction().unwrap();
        storage::store(&file, &source, "", &transaction).unwrap();
        transaction.commit().unwrap();
    }

    pub fn remove(&mut self, source: Location) {
        let transaction = self.connection.transaction().unwrap();
        storage::remove(&source, &transaction).unwrap();
        transaction.commit().unwrap();
    }
}
