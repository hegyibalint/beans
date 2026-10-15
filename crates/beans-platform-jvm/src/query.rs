use beans_core::{location::Location, query_scope::QueryScope, revision::RevisionEntry};

use crate::JvmPlatform;

impl JvmPlatform {
    pub fn find_classes(
        &self,
        _scope: QueryScope<'_>,
        _binary_name: &str,
    ) -> Vec<RevisionEntry<Location>> {
        // JVM persistence is deferred while Java moves to SQLite.
        Vec::new()
    }
}
