//! Java lowering and analysis, independent of storage and editor protocols.

use std::path::Path;

use beans_core::resource::ResourceId;

use crate::model::File;

pub mod codec;
pub mod features;
pub mod lowering;
pub mod model;
pub mod model_query;
pub mod query;
pub mod semantics;

pub use lowering::lower_into;

pub fn accept(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "java")
}

pub fn process(_source: &ResourceId, contents: &str) -> Option<File> {
    Some(lower_into(contents))
}

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests;
