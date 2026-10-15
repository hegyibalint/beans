use beans_core::{location::Location, query_scope::QueryScope};

use crate::JvmPlatform;

impl JvmPlatform {
    pub fn find_classes(&self, _scope: QueryScope<'_>, _binary_name: &str) -> Vec<Location> {
        // JVM persistence is deferred while Java moves to SQLite.
        Vec::new()
    }
}
