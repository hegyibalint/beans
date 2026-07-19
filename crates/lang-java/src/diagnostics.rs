use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};

use crate::model::JavaFile;

/// Milestone scaffolding: flag every type reference until resolution can
/// sort them into resolved / importable / unresolvable.
pub fn dummy_diagnostic(model: &JavaFile) -> Vec<Diagnostics> {
    todo!("Do this after the scopes are implemented")
}
