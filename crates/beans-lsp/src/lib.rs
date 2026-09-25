//! LSP lifecycle and protocol dispatch around the Beans engine.

mod features;
mod model;
mod server;

pub use server::{Server, run};
