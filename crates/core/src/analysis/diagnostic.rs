use crate::model::OffsetSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

pub struct Diagnostics {
    pub span: OffsetSpan,
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub message: String,
}
