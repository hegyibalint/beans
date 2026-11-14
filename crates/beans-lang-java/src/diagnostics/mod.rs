use beans_core::{location::Location, location_match::LocationMatcher};
use rusqlite::Connection;

use crate::{engine::JavaLanguage, model::File, query};

mod unresolved_types;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: beans_core::ranges::ByteRange,
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
}

impl JavaLanguage {
    pub fn diagnostics(
        &self,
        source: &Location,
        match_locations: Option<&LocationMatcher<'_>>,
        connection: &Connection,
    ) -> Result<Vec<Diagnostic>, String> {
        let Some(file) = query::file(source, connection)? else {
            return Ok(Vec::new());
        };
        self.diagnostics_for_file(&file, source, match_locations, connection)
    }

    pub fn diagnostics_for_file(
        &self,
        file: &File,
        source: &Location,
        match_locations: Option<&LocationMatcher<'_>>,
        connection: &Connection,
    ) -> Result<Vec<Diagnostic>, String> {
        unresolved_types::collect(file, source, match_locations, connection)
    }
}
