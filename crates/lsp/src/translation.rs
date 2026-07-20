use std::path::PathBuf;

use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};
use beans_core::model::Span;
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

pub fn position_to_offset(contents: &str, position: Position) -> Option<usize> {
    let mut line_start = 0;
    for _ in 0..position.line {
        let newline = contents[line_start..].find('\n')?;
        line_start += newline + 1;
    }

    let mut line_end = contents[line_start..]
        .find('\n')
        .map_or(contents.len(), |newline| line_start + newline);
    if line_end > line_start && contents.as_bytes()[line_end - 1] == b'\r' {
        line_end -= 1;
    }

    let line = &contents[line_start..line_end];
    let character = usize::try_from(position.character).ok()?;
    if line.is_ascii() {
        return (character <= line.len()).then_some(line_start + character);
    }

    let mut utf16_offset = 0;
    for (byte_offset, character_value) in line.char_indices() {
        if utf16_offset == character {
            return Some(line_start + byte_offset);
        }

        utf16_offset += character_value.len_utf16();
        if utf16_offset > character {
            return None;
        }
    }

    (utf16_offset == character).then_some(line_end)
}

/// Translates an internal diagnostic into its `lsp_types` counterpart.
pub fn translate_diagnostics(contents: &str, diagnostic: &Diagnostics) -> lsp_types::Diagnostic {
    lsp_types::Diagnostic {
        range: span_to_range(contents, &diagnostic.span),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn source_of(raw: &str) -> Option<JvmSource> {
        uri_to_source(&raw.parse().expect("valid uri"))
    }

    #[test]
    fn position_to_offset_finds_ascii_lines_and_columns() {
        let contents = "first\nsecond\n";

        assert_eq!(position_to_offset(contents, Position::new(1, 3)), Some(9));
        assert_eq!(
            position_to_offset(contents, Position::new(2, 0)),
            Some(contents.len())
        );
    }

    #[test]
    fn position_to_offset_counts_utf16_code_units() {
        let contents = "a😀b";

        assert_eq!(position_to_offset(contents, Position::new(0, 1)), Some(1));
        assert_eq!(position_to_offset(contents, Position::new(0, 3)), Some(5));
        assert_eq!(position_to_offset(contents, Position::new(0, 4)), Some(6));
        assert_eq!(position_to_offset(contents, Position::new(0, 2)), None);
    }

    #[test]
    fn position_to_offset_handles_multibyte_bmp_characters() {
        assert_eq!(position_to_offset("éx", Position::new(0, 1)), Some(2));
    }

    #[test]
    fn position_to_offset_rejects_positions_outside_the_document() {
        assert_eq!(position_to_offset("abc", Position::new(0, 4)), None);
        assert_eq!(position_to_offset("abc", Position::new(1, 0)), None);
    }

    #[test]
    fn position_to_offset_excludes_the_carriage_return_from_crlf_lines() {
        let contents = "ab\r\nc";

        assert_eq!(position_to_offset(contents, Position::new(0, 2)), Some(2));
        assert_eq!(position_to_offset(contents, Position::new(1, 0)), Some(4));
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
