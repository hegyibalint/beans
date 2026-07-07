use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};

use crate::model::JavaFile;

pub fn dummy_diagnostics(model: &JavaFile) -> Option<Diagnostics> {
    let package_span = model.package.unwrap().span;

    return Some(Diagnostics {
        span: package_span,
        message: "dummy diagnostics".to_string(),
        code: "dummy_diag",
        severity: DiagnosticSeverity::Warning,
    });
}
