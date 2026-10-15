//! JVM container processing and model interpretation, independent of storage.

pub mod codec;
pub mod lowering;
mod model;
pub mod model_query;
mod query;

#[cfg(test)]
mod tests;

pub use crate::{lowering::process, model::class::Class, query::find_classes};
