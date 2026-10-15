//! LSP protocol handling and adapters for Engine language features.

mod features;
mod model;
mod server;

pub use server::{Server, run};
