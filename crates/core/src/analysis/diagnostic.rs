use crate::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

pub struct Diagnostics {
    pub span: Span,
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub message: String,
}
