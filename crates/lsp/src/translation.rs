use std::path::PathBuf;

use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};
use beans_core::model::{LineColumnPosition, LineColumnSpan};
use beans_platform_jvm::model::JvmSource;
use lsp_types::{Position, Uri};

/// Only `file:` URIs name something we can read; `untitled:`, `git:` and the
/// virtual-filesystem schemes have no path behind them.
pub fn uri_to_source(uri: &Uri) -> Option<JvmSource> {
    if !uri.scheme()?.as_str().eq_ignore_ascii_case("file") {
        return None;
    }
    let path = uri.path().as_estr().decode().into_string_lossy();
    Some(JvmSource::SourceFile {
        path: PathBuf::from(path.as_ref()),
    })
}

/// The inverse of `uri_to_source` for on-disk sources. Only `SourceFile`
/// names a real path; the virtual JVM sources have no `file:` URI.
pub fn source_to_uri(source: &JvmSource) -> Option<Uri> {
    match source {
        JvmSource::SourceFile { path } => format!("file://{}", path.to_str()?).parse().ok(),
        _ => None,
    }
}

/// The line/column an editor sends us, in our own coordinate type. The engine
/// turns it into a byte offset — the LSP layer itself holds no text.
pub fn position_to_line_column(position: Position) -> LineColumnPosition {
    LineColumnPosition {
        line: position.line,
        character: position.character,
    }
}

pub fn text_range_to_range(range: LineColumnSpan) -> lsp_types::Range {
    lsp_types::Range {
        start: line_column_to_position(range.start),
        end: line_column_to_position(range.end),
    }
}

fn line_column_to_position(position: LineColumnPosition) -> Position {
    Position {
        line: position.line,
        character: position.character,
    }
}

/// Translates an internal diagnostic into its `lsp_types` counterpart. The
/// range is computed by the engine from the file's text and handed in, so the
/// translation itself stays text-free.
pub fn translate_diagnostics(
    range: lsp_types::Range,
    diagnostic: &Diagnostics,
) -> lsp_types::Diagnostic {
    lsp_types::Diagnostic {
        range,
        severity: Some(translate_severity(diagnostic.severity)),
        code: Some(lsp_types::NumberOrString::String(
            diagnostic.code.to_string(),
        )),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn source_of(raw: &str) -> Option<JvmSource> {
        uri_to_source(&raw.parse().expect("valid uri"))
    }

    #[test]
    fn file_uri_becomes_a_source_file() {
        assert_eq!(
            source_of("file:///home/beans/Foo.java"),
            Some(JvmSource::SourceFile {
                path: PathBuf::from("/home/beans/Foo.java"),
            })
        );
    }

    #[test]
    fn percent_escapes_are_decoded() {
        assert_eq!(
            source_of("file:///home/my%20project/Foo.java"),
            Some(JvmSource::SourceFile {
                path: PathBuf::from("/home/my project/Foo.java"),
            })
        );
    }

    #[test]
    fn pathless_schemes_have_no_source() {
        assert_eq!(source_of("untitled:Untitled-1"), None);
        assert_eq!(source_of("git:/home/beans/Foo.java?%7B%7D"), None);
    }
}
