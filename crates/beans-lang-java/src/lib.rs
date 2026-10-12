//! Java lowering and analysis, independent of storage and editor protocols.

use std::path::Path;

use beans_core::{query_scope::QueryScope, resource::ResourceId};
use beans_storage::RevisionEntry;

use crate::{model::File, model_query::JavaModelQuery, query::JavaQuery};

pub mod features;
pub mod lowering;
pub mod model;
pub mod model_query;
pub mod query;
pub mod semantics;

pub use lowering::lower_into;

#[derive(Default)]
pub struct JavaLanguage;

impl JavaLanguage {
    pub fn new() -> Self {
        Self
    }

    pub fn accept(&self, path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == "java")
    }

    /// The caller selects Java; a resource's name does not override that selection.
    pub fn process(&self, _source: &ResourceId, contents: &str) -> Option<File> {
        Some(lower_into(contents))
    }

    pub fn query<'a, Q: JavaModelQuery>(
        &self,
        models: &'a Q,
        scope: QueryScope<'a>,
    ) -> JavaQuery<'a, Q> {
        JavaQuery::new(models, scope)
    }

    pub fn analyse(&self, _entry: &RevisionEntry<ResourceId>, _scope: QueryScope<'_>) {
        todo!("analysis is not implemented")
    }
}

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests;
