use beans_core::{location::Location, query_scope::QueryScope};
use beans_platform_jvm::JvmPlatform;
use rusqlite::Connection;

use crate::{
    engine::JavaLanguage, model::File, query::JavaQuery,
    semantics::resolution::query::ResolutionQuery,
};

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
        connection: &Connection,
        scope: QueryScope<'_>,
        jvm: &JvmPlatform,
        source: &Location,
    ) -> Result<Vec<Diagnostic>, String> {
        let java = JavaQuery::new(connection, scope);
        let Some(file) = java.file(source)? else {
            return Ok(Vec::new());
        };
        self.diagnostics_for_file(&java, jvm, source, &file)
    }

    pub fn diagnostics_for_file(
        &self,
        java: &JavaQuery<'_>,
        jvm: &JvmPlatform,
        source: &Location,
        file: &File,
    ) -> Result<Vec<Diagnostic>, String> {
        let definitions = ResolutionQuery::new(java, jvm);
        unresolved_types::collect(file, source, &definitions)
    }
}
