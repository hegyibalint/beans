pub mod diagnostics;
pub mod engine;
pub mod features;
pub mod lowering;
pub mod model;
pub mod query;
pub mod semantics;
pub mod storage;

pub use lowering::lower_into;

#[cfg(test)]
extern crate self as beans_lang_java;
#[cfg(test)]
mod testing;
