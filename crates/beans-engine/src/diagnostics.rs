use beans_core::location::Location;

pub use beans_lang_java::diagnostics::{Diagnostic, DiagnosticSeverity};

use crate::{Engine, scopes::load_scopes};

impl Engine {
    pub fn diagnostics(&self, source: &Location) -> Result<Vec<Diagnostic>, String> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let scopes = load_scopes(&transaction)?;
        self.java.diagnostics(source, &scopes, &transaction)
    }
}
