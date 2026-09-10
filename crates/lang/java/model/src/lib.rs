pub mod declarations;
mod file;
pub mod imports;
pub mod names;
pub mod references;
pub mod scopes;

pub use file::{DeclarationEntry, File, ScopeEntry};
