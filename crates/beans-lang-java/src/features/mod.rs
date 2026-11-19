//! Java implementations of protocol-neutral editor features.

use beans_core::location_match::LocationMatcher;
use rusqlite::Connection;

mod hover;
mod navigation;
mod semantic_tokens;

/// Binds Java feature queries to a database snapshot and an optional location matcher.
#[derive(Clone, Copy)]
pub struct JavaFeatureContext<'a> {
    pub connection: &'a Connection,
    pub location_matcher: Option<&'a LocationMatcher<'a>>,
    pub workspace_root: Option<&'a std::path::Path>,
}
