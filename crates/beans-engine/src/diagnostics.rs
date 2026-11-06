use beans_core::{compilation_scope::CompilationScopes, location::Location};

pub use beans_lang_java::diagnostics::{Diagnostic, DiagnosticSeverity};

use crate::Engine;

impl Engine {
    pub fn diagnostics(&self, source: &Location) -> Result<Vec<Diagnostic>, String> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        self.java
            .diagnostics(source, &CompilationScopes::default(), &transaction)
    }
}
