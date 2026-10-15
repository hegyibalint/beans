use beans_core::{location::Location, query_scope::QueryScope};
use beans_storage::RevisionEntry;

use crate::JvmPlatform;

#[cfg(test)]
mod tests;

impl JvmPlatform {
    /// Looks up an exact binary name in internal form (JVMS §4.2.1).
    pub fn find_classes(
        &self,
        scope: QueryScope<'_>,
        binary_name: &str,
    ) -> Vec<RevisionEntry<Location>> {
        let mut found = Vec::new();
        for (resource, class) in self.classes.iter(scope.revision()) {
            if class.name != binary_name || !scope.classpath().contains_class(resource) {
                continue;
            }
            found.push(RevisionEntry {
                revision: scope.revision(),
                key: resource.clone(),
            });
        }
        found
    }
}
