use beans_core::{location::Location, location_match::LocationMatcher};

pub use beans_lang_java::diagnostics::{Diagnostic, DiagnosticSeverity};

use crate::{Engine, modules::load_modules};

impl Engine {
    pub fn diagnostics(&self, source: &Location) -> Result<Vec<Diagnostic>, String> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let modules = load_modules(&transaction)?;
        let location_matcher = LocationMatcher::new(&modules);
        self.java
            .diagnostics(source, Some(&location_matcher), &transaction)
    }
}
