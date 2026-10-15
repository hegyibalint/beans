use beans_core::{classpath::Unrestricted, query_scope::QueryScope, resource::ResourceId};

pub use beans_lang_java::diagnostics::{Diagnostic, DiagnosticSeverity};

use crate::Engine;

impl Engine {
    pub fn diagnostics(&self, source: &ResourceId) -> Vec<Diagnostic> {
        self.java.diagnostics(
            QueryScope::new(self.revision, &Unrestricted),
            &self.jvm,
            source,
        )
    }
}
