use beans_core::{compilation_scope::CompilationScopes, location::Location};

pub use beans_lang_java::diagnostics::{Diagnostic, DiagnosticSeverity};

use crate::Engine;

impl Engine {
    pub fn diagnostics(&self, source: &Location) -> Result<Vec<Diagnostic>, String> {
        let transaction = self
            .storage
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        self.java
            .diagnostics(source, &CompilationScopes::unrestricted(), &transaction)
    }
}
