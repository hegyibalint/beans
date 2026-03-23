use std::path::{Path, PathBuf};

/// A cursor position extracted from a Java fixture file.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorPosition {
    /// None for anonymous `<cur>`, Some("name") for `<cur:name>`
    pub name: Option<String>,
    /// The file this cursor was found in
    pub file: PathBuf,
    /// 0-based line number in the stripped source
    pub line: u32,
    /// 0-based column number in the stripped source
    pub col: u32,
}

/// Result of stripping cursor markers from source code.
#[derive(Debug, Clone)]
pub struct StrippedSource {
    /// The source with all cursor markers removed
    pub clean: String,
    /// All cursor positions found
    pub cursors: Vec<CursorPosition>,
}

/// Strip `<cur>` and `<cur:name>` markers from Java source code.
///
/// Returns the cleaned source and a list of cursor positions.
/// Positions are recorded in the stripped output coordinates.
///
/// # Panics
/// Panics if more than one anonymous `<cur>` is found in a single file.
/// Panics if duplicate cursor names are found.
pub fn strip_markers(source: &str, file: &Path) -> StrippedSource {
    let mut clean = String::with_capacity(source.len());
    let mut cursors: Vec<CursorPosition> = Vec::new();
    let mut line: u32 = 0;
    let mut col: u32 = 0;

    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' && i + 4 < len && &bytes[i + 1..i + 4] == b"cur" {
            // Could be <cur> or <cur:NAME>
            if i + 4 < len && bytes[i + 4] == b'>' {
                // Anonymous cursor: <cur>
                let has_anonymous = cursors.iter().any(|c| c.name.is_none());
                assert!(!has_anonymous, "duplicate anonymous <cur> marker");

                cursors.push(CursorPosition {
                    name: None,
                    file: file.to_path_buf(),
                    line,
                    col,
                });
                i += 5; // skip "<cur>"
                continue;
            } else if i + 4 < len && bytes[i + 4] == b':' {
                // Named cursor: <cur:NAME>
                let name_start = i + 5;
                if let Some(end_offset) = bytes[name_start..].iter().position(|&b| b == b'>') {
                    let name_end = name_start + end_offset;
                    let name = String::from_utf8_lossy(&bytes[name_start..name_end]).to_string();

                    let has_duplicate = cursors.iter().any(|c| c.name.as_deref() == Some(&name));
                    assert!(!has_duplicate, "duplicate cursor name: {name}");

                    cursors.push(CursorPosition {
                        name: Some(name),
                        file: file.to_path_buf(),
                        line,
                        col,
                    });
                    i = name_end + 1; // skip past '>'
                    continue;
                }
            }
        }

        // Regular character — emit it
        let ch = bytes[i];
        clean.push(ch as char);
        if ch == b'\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        i += 1;
    }

    StrippedSource { clean, cursors }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anonymous_cursor() {
        let result = strip_markers("public class <cur>Foo {}", Path::new("Foo.java"));
        assert_eq!(result.clean, "public class Foo {}");
        assert_eq!(result.cursors.len(), 1);
        assert_eq!(result.cursors[0].name, None);
        assert_eq!(result.cursors[0].line, 0);
        assert_eq!(result.cursors[0].col, 13);
    }

    #[test]
    fn test_named_cursor() {
        let result = strip_markers("public class <cur:class_def>Foo {}", Path::new("Foo.java"));
        assert_eq!(result.clean, "public class Foo {}");
        assert_eq!(result.cursors.len(), 1);
        assert_eq!(result.cursors[0].name, Some("class_def".to_string()));
        assert_eq!(result.cursors[0].line, 0);
        assert_eq!(result.cursors[0].col, 13);
    }

    #[test]
    fn test_multiple_named_cursors() {
        let src = "<cur:pkg>package com.example;\n<cur:cls>public class Foo {}";
        let result = strip_markers(src, Path::new("Foo.java"));
        assert_eq!(result.clean, "package com.example;\npublic class Foo {}");
        assert_eq!(result.cursors.len(), 2);
        assert_eq!(result.cursors[0].name, Some("pkg".to_string()));
        assert_eq!(result.cursors[0].line, 0);
        assert_eq!(result.cursors[0].col, 0);
        assert_eq!(result.cursors[1].name, Some("cls".to_string()));
        assert_eq!(result.cursors[1].line, 1);
        assert_eq!(result.cursors[1].col, 0);
    }

    #[test]
    fn test_no_cursors() {
        let result = strip_markers("public class Foo {}", Path::new("Foo.java"));
        assert_eq!(result.clean, "public class Foo {}");
        assert_eq!(result.cursors.len(), 0);
    }

    #[test]
    fn test_cursor_preserves_multiline() {
        let src = "line1\nline2\n<cur>line3";
        let result = strip_markers(src, Path::new("test.java"));
        assert_eq!(result.clean, "line1\nline2\nline3");
        assert_eq!(result.cursors[0].line, 2);
        assert_eq!(result.cursors[0].col, 0);
    }

    #[test]
    #[should_panic]
    fn test_duplicate_anonymous_panics() {
        strip_markers("<cur>foo <cur>bar", Path::new("test.java"));
    }

    #[test]
    #[should_panic]
    fn test_duplicate_names_panics() {
        strip_markers("<cur:x>foo <cur:x>bar", Path::new("test.java"));
    }

    #[test]
    fn test_mixed_anonymous_and_named() {
        let src = "<cur>foo <cur:named>bar";
        let result = strip_markers(src, Path::new("test.java"));
        assert_eq!(result.clean, "foo bar");
        assert_eq!(result.cursors.len(), 2);
        assert_eq!(result.cursors[0].name, None);
        assert_eq!(result.cursors[1].name, Some("named".to_string()));
    }
}
