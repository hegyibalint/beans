//! A Java-like declaration view generated from a stored JVM class model.

mod render;

pub use render::{DeclarationRanges, RenderedFile, generate};
