use beans_core::{classpath::Unrestricted, location::Location, query_scope::QueryScope};

pub use beans_lang_java::diagnostics::{Diagnostic, DiagnosticSeverity};

use crate::Engine;

impl Engine {
    pub fn diagnostics(&self, source: &Location) -> Vec<Diagnostic> {
        self.java.diagnostics(
            QueryScope::new(self.revision, &Unrestricted),
            &self.jvm,
            source,
        )
    }
}
