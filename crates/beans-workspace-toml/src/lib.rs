//! A `beans.toml` workspace backend, independent of language state and editor protocols.
//!
//! ```
//! use std::path::Path;
//! use beans_workspace::Workspaces;
//! use beans_workspace_toml::parse;
//!
//! let backend = parse(
//!     "[unit.app]\nsources = [\"src\"]\n",
//!     Path::new("/project"),
//! )?;
//! let workspaces = Workspaces::new(backend);
//! let classpath = workspaces.classpath_for("file:///project/src/Example.java");
//! # Ok::<(), beans_workspace_toml::ParseError>(())
//! ```

pub mod engine;
pub mod model;
pub mod parser;

pub use engine::TomlWorkspace;
pub use parser::{DESCRIPTOR, LoadError, ParseError, load, parse};

mod paths;
