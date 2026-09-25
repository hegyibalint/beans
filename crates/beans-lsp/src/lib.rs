//! LSP lifecycle and protocol dispatch around the Beans engine.

mod open_document;
mod server;
mod session;

pub use server::{Server, run};
