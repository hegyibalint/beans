use beans_core::{query_scope::QueryScope, resource::ResourceId};
use beans_storage::RevisionEntry;

use crate::model_query::JvmModelQuery;

/// Looks up an exact binary name in internal form (JVMS §4.2.1).
pub fn find_classes(
    models: &impl JvmModelQuery,
    scope: QueryScope<'_>,
    binary_name: &str,
) -> Result<Vec<RevisionEntry<ResourceId>>, String> {
    let mut found = Vec::new();
    for (resource, class) in models.jvm_classes()? {
        if class.name != binary_name || !scope.classpath().contains_class(&resource) {
            continue;
        }
        found.push(RevisionEntry {
            revision: scope.revision(),
            key: resource,
        });
    }
    Ok(found)
}

#[cfg(test)]
mod tests;
