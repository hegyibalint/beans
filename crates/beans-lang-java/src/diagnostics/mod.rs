use beans_core::{query_scope::QueryScope, ranges::ByteRange, resource::ResourceId};
use beans_platform_jvm::JvmPlatform;

use crate::{engine::JavaLanguage, semantics::resolution::query::ResolutionQuery};

mod unresolved_types;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: ByteRange,
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
        scope: QueryScope<'_>,
        jvm: &JvmPlatform,
        source: &ResourceId,
    ) -> Vec<Diagnostic> {
        let java = self.query(scope);
        let Some(file) = java.file(source) else {
            return Vec::new();
        };
        let definitions = ResolutionQuery::new(&java, jvm);
        unresolved_types::collect(file, scope.revision(), source, &definitions)
    }
}
