use beans_core::{classpath::Unrestricted, location::Location, query_scope::QueryScope};

pub use beans_lang_java::diagnostics::{Diagnostic, DiagnosticSeverity};

use crate::Engine;

impl Engine {
    pub fn diagnostics(&self, source: &Location) -> Result<Vec<Diagnostic>, String> {
        let transaction = self
            .storage
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        self.java.diagnostics(
            &transaction,
            QueryScope::new(self.revision, &Unrestricted),
            &self.jvm,
            source,
        )
    }
}
