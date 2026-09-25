pub mod features;
mod operations;

use beans_core::model::ranges::ByteRange;

use crate::model::File;

/// Finds a modeled type identifier at a byte without resolving its declaration.
pub fn type_reference_at(file: &File, offset: usize) -> Option<ByteRange> {
    operations::type_reference_at(file, offset).map(|occurrence| occurrence.range)
}
