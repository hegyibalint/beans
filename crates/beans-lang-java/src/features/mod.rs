//! Java implementations of protocol-neutral editor features.

use beans_core::compilation_scope::CompilationScopes;
use rusqlite::Connection;

mod hover;
mod navigation;
mod semantic_tokens;

/// Binds Java feature queries to a database snapshot and scope.
#[derive(Clone, Copy)]
pub struct JavaFeatureContext<'a> {
    pub connection: &'a Connection,
    pub scopes: &'a CompilationScopes,
    pub workspace_root: Option<&'a std::path::Path>,
}
