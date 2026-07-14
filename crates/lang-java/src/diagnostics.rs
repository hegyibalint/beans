use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};

use crate::model::JavaFile;

/// Milestone scaffolding: flag every type reference until resolution can
/// sort them into resolved / importable / unresolvable.
pub fn symbol_diagnostics(model: &JavaFile) -> Vec<Diagnostics> {
    model
        .type_references()
        .map(|reference| Diagnostics {
            span: reference.span,
            severity: DiagnosticSeverity::Warning,
            code: "type-reference",
            message: format!("type reference: {}", reference.dotted()),
        })
        .collect()
}
