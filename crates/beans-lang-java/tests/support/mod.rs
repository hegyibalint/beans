#![allow(dead_code)]

use beans_core::{location::Location, query_scope::QueryScope};
use beans_lang_java::{engine::JavaLanguage, model::File, query::JavaQuery, storage};
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
        storage::store(&transaction, &file, &source, "").unwrap();
        transaction.commit().unwrap();
    }

    pub fn remove(&mut self, source: Location) {
        let transaction = self.connection.transaction().unwrap();
        storage::remove(&transaction, &source).unwrap();
        transaction.commit().unwrap();
    }

    pub fn query<'a>(&'a self, scope: QueryScope<'a>) -> JavaQuery<'a> {
        JavaQuery::new(&self.connection, scope)
    }
}
