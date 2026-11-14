use beans_core::location::Location;

pub use beans_lang_java::diagnostics::{Diagnostic, DiagnosticSeverity};

use crate::{Engine, location_matches::matcher, modules::load_modules};

impl Engine {
    pub fn diagnostics(&self, source: &Location) -> Result<Vec<Diagnostic>, String> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let modules = load_modules(&transaction)?;
        let match_locations = matcher(&modules);
        self.java
            .diagnostics(source, Some(&match_locations), &transaction)
    }
}
