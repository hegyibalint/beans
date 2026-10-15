use beans_core::{
    classpath::Unrestricted,
    location::Location,
    query_scope::QueryScope,
    revision::{Revision, RevisionEntry},
};
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
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(include_str!("../../beans-engine/src/storage/schema.sql"))
            .unwrap();
        connection.execute_batch(storage::SCHEMA).unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        Self { connection }
    }

    pub fn process(&self, contents: &str) -> File {
        JavaLanguage::new().process(contents)
    }

    pub fn store(
        &mut self,
        revision: Revision,
        file: File,
        source: Location,
    ) -> RevisionEntry<Location> {
        let transaction = self.connection.transaction().unwrap();
        storage::store(&transaction, revision, &file, &source, "").unwrap();
        transaction.commit().unwrap();
        RevisionEntry {
            revision,
            key: source,
        }
    }

    pub fn remove(&mut self, revision: Revision, source: Location) {
        let transaction = self.connection.transaction().unwrap();
        storage::remove(&transaction, revision, &source).unwrap();
        transaction.commit().unwrap();
    }

    pub fn query<'a>(&'a self, scope: QueryScope<'a>) -> JavaQuery<'a> {
        JavaQuery::new(&self.connection, scope)
    }

    pub fn file(&self, revision: Revision, source: &Location) -> Option<File> {
        self.query(QueryScope::new(revision, &Unrestricted))
            .file(source)
            .unwrap()
    }
}
