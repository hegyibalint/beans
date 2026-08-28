use std::path::PathBuf;

use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};
use beans_core::language::{CompletionItem, CompletionItemKind};
use beans_core::model::{LineColumnPosition, LineColumnSpan};
use beans_platform_jvm as jvm;
use lsp_types::{Position, Uri};

pub fn uri_to_source(uri: &Uri) -> Option<jvm::model::Source> {
    Some(jvm::model::Source::SourceFile {
        path: uri_to_path(uri)?,
    })
}

/// Only `file:` URIs name something we can read; `untitled:`, `git:` and the
/// virtual-filesystem schemes have no path behind them.
pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    if !uri.scheme()?.as_str().eq_ignore_ascii_case("file") {
        return None;
    }
    let path = uri.path().as_estr().decode().into_string_lossy();
    Some(PathBuf::from(path.as_ref()))
}

/// The inverse of `uri_to_source` for on-disk sources. Only `SourceFile`
/// names a real path; the virtual JVM sources have no `file:` URI.
pub fn source_to_uri(source: &jvm::model::Source) -> Option<Uri> {
    match source {
        jvm::model::Source::SourceFile { path } => {
            format!("file://{}", path.to_str()?).parse().ok()
        }
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

/// Translates one completion row. The range it overwrites is computed by the
/// engine from the file's text and handed in, so this stays text-free like
/// every other translation here.
///
/// The handle is not sent yet. It exists so the item's shape is right; nothing
/// asks for a row to be resolved until there is something expensive to say.
pub fn translate_completion_item(
    replace: Option<lsp_types::Range>,
    item: &CompletionItem<jvm::model::Source>,
) -> lsp_types::CompletionItem {
    lsp_types::CompletionItem {
        label: item.label.clone(),
        kind: Some(translate_completion_kind(item.kind)),
        detail: item.detail.clone(),
        text_edit: replace.map(|range| {
            lsp_types::CompletionTextEdit::Edit(lsp_types::TextEdit {
                range,
                new_text: item.insert.clone(),
            })
        }),
        // A client filters on the label unless told otherwise, and a Java
        // method's label carries its parameters. Without this, typing `desc`
        // still matches `describe(int factor)` but a second keystroke past the
        // name would not.
        filter_text: Some(item.insert.clone()),
        ..Default::default()
    }
}

/// LSP's vocabulary is not ours and does not have to be: it has no record and
/// no annotation interface, and it draws parameters with the variable icon.
fn translate_completion_kind(kind: CompletionItemKind) -> lsp_types::CompletionItemKind {
    match kind {
        CompletionItemKind::Class => lsp_types::CompletionItemKind::CLASS,
        CompletionItemKind::Interface => lsp_types::CompletionItemKind::INTERFACE,
        CompletionItemKind::Enum => lsp_types::CompletionItemKind::ENUM,
        CompletionItemKind::Record => lsp_types::CompletionItemKind::STRUCT,
        CompletionItemKind::AnnotationInterface => lsp_types::CompletionItemKind::INTERFACE,
        CompletionItemKind::Method => lsp_types::CompletionItemKind::METHOD,
        CompletionItemKind::Field => lsp_types::CompletionItemKind::FIELD,
        CompletionItemKind::Variable | CompletionItemKind::Parameter => {
            lsp_types::CompletionItemKind::VARIABLE
        }
        CompletionItemKind::TypeParameter => lsp_types::CompletionItemKind::TYPE_PARAMETER,
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

    fn source_of(raw: &str) -> Option<jvm::model::Source> {
        uri_to_source(&raw.parse().expect("valid uri"))
    }

    #[test]
    fn file_uri_becomes_a_source_file() {
        assert_eq!(
            source_of("file:///home/beans/Foo.java"),
            Some(jvm::model::Source::SourceFile {
                path: PathBuf::from("/home/beans/Foo.java"),
            })
        );
    }

    #[test]
    fn percent_escapes_are_decoded() {
        assert_eq!(
            source_of("file:///home/my%20project/Foo.java"),
            Some(jvm::model::Source::SourceFile {
                path: PathBuf::from("/home/my project/Foo.java"),
            })
        );
    }

    /// The row a user reads and the text a user gets are different strings for
    /// a method, so accepting `describe(int factor)` has to type `describe`.
    /// Before the two were split the label was both, and a label with
    /// parameters in it would have written them into the buffer.
    #[test]
    fn a_row_inserts_what_it_says_it_inserts_rather_than_its_label() {
        let range = lsp_types::Range {
            start: Position {
                line: 1,
                character: 8,
            },
            end: Position {
                line: 1,
                character: 12,
            },
        };
        let item: CompletionItem<jvm::model::Source> = CompletionItem {
            label: "describe(int factor)".to_string(),
            insert: "describe".to_string(),
            kind: CompletionItemKind::Method,
            detail: Some("void".to_string()),
            replace: beans_core::model::OffsetSpan {
                start: beans_core::model::Offset(0),
                end: beans_core::model::Offset(4),
            },
            handle: None,
        };

        let translated = translate_completion_item(Some(range), &item);

        assert_eq!(translated.label, "describe(int factor)");
        let Some(lsp_types::CompletionTextEdit::Edit(edit)) = translated.text_edit else {
            panic!("a row with a range carries a text edit");
        };
        assert_eq!(edit.new_text, "describe");
        // Filtering follows the label unless told otherwise, and the label is
        // no longer something the user is typing.
        assert_eq!(translated.filter_text.as_deref(), Some("describe"));
    }

    #[test]
    fn pathless_schemes_have_no_source() {
        assert_eq!(source_of("untitled:Untitled-1"), None);
        assert_eq!(source_of("git:/home/beans/Foo.java?%7B%7D"), None);
    }
}
