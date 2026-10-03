mod descriptor;
mod error;

pub use descriptor::{DESCRIPTOR, load, parse};
pub use error::{LoadError, ParseError};

#[cfg(test)]
mod tests;
