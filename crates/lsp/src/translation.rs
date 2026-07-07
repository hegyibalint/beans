use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};
use beans_core::Span;

/// Translates an internal diagnostic into its `lsp_types` counterpart.
pub fn translate_diagnostics(contents: &str, diagnostic: &Diagnostics) -> lsp_types::Diagnostic {
    lsp_types::Diagnostic {
        range: span_to_range(contents, &diagnostic.span),
        severity: Some(translate_severity(diagnostic.severity)),
        code: Some(lsp_types::NumberOrString::String(diagnostic.code.to_string())),
        source: Some("beans".to_string()),
        message: diagnostic.message.clone(),
        ..Default::default()
    }
}

fn translate_severity(severity: DiagnosticSeverity) -> lsp_types::DiagnosticSeverity {
    match severity {
        DiagnosticSeverity::Error => lsp_types::DiagnosticSeverity::ERROR,
        DiagnosticSeverity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        DiagnosticSeverity::Info => lsp_types::DiagnosticSeverity::INFORMATION,
        DiagnosticSeverity::Hint => lsp_types::DiagnosticSeverity::HINT,
    }
}

pub fn span_to_range(contents: &str, span: &Span) -> lsp_types::Range {
    lsp_types::Range {
        start: offset_to_position(contents, span.start),
        end: offset_to_position(contents, span.end),
    }
}

fn offset_to_position(contents: &str, offset: usize) -> lsp_types::Position {
    let before = &contents[..offset];
    let line = before.bytes().filter(|&b| b == b'\n').count();
    let line_start = before.rfind('\n').map_or(0, |nl| nl + 1);
    let character = contents[line_start..offset].encode_utf16().count();
    lsp_types::Position {
        line: line as u32,
        character: character as u32,
    }
}
