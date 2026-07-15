use std::path::{Path, PathBuf};

/// A `<cur>` or `<cur:name>` position in a fixture file, as a byte offset
/// into the stripped source — the same coordinates as `Span`.
#[derive(Debug, Clone, PartialEq)]
pub struct Cursor {
    pub name: Option<String>,
    pub file: PathBuf,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct StrippedSource {
    pub clean: String,
    pub cursors: Vec<Cursor>,
}

pub fn strip_markers(source: &str, file: &Path) -> StrippedSource {
    let mut clean = String::with_capacity(source.len());
    let mut cursors: Vec<Cursor> = Vec::new();
    let mut rest = source;

    while let Some(open) = rest.find("<cur") {
        let after = &rest[open + 4..];
        let (name, marker_len) = if after.starts_with('>') {
            (None, 5)
        } else if let Some(close) = after.starts_with(':').then(|| after.find('>')).flatten() {
            (Some(after[1..close].to_string()), 4 + close + 1)
        } else {
            clean.push_str(&rest[..open + 4]);
            rest = &rest[open + 4..];
            continue;
        };

        assert!(
            !cursors.iter().any(|c| c.name == name),
            "duplicate cursor marker {name:?} in {}",
            file.display()
        );
        clean.push_str(&rest[..open]);
        cursors.push(Cursor {
            name,
            file: file.to_path_buf(),
            offset: clean.len(),
        });
        rest = &rest[open + marker_len..];
    }
    clean.push_str(rest);

    StrippedSource { clean, cursors }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anonymous_cursor() {
        let result = strip_markers("class <cur>Foo {}", Path::new("Foo.java"));
        assert_eq!(result.clean, "class Foo {}");
        assert_eq!(result.cursors.len(), 1);
        assert_eq!(result.cursors[0].name, None);
        assert_eq!(result.cursors[0].offset, 6);
    }

    #[test]
    fn named_cursor() {
        let result = strip_markers("class <cur:def>Foo {}", Path::new("Foo.java"));
        assert_eq!(result.clean, "class Foo {}");
        assert_eq!(result.cursors[0].name.as_deref(), Some("def"));
        assert_eq!(result.cursors[0].offset, 6);
    }

    #[test]
    fn multiple_cursors_keep_offsets_in_stripped_coordinates() {
        let result = strip_markers("<cur:a>foo <cur:b>bar", Path::new("Foo.java"));
        assert_eq!(result.clean, "foo bar");
        assert_eq!(result.cursors[0].offset, 0);
        assert_eq!(result.cursors[1].offset, 4);
    }

    #[test]
    fn no_cursors() {
        let result = strip_markers("class Foo {}", Path::new("Foo.java"));
        assert_eq!(result.clean, "class Foo {}");
        assert!(result.cursors.is_empty());
    }

    #[test]
    fn non_marker_angle_text_passes_through() {
        let result = strip_markers("List<current> x;", Path::new("Foo.java"));
        assert_eq!(result.clean, "List<current> x;");
        assert!(result.cursors.is_empty());
    }

    #[test]
    #[should_panic(expected = "duplicate cursor")]
    fn duplicate_names_panic() {
        strip_markers("<cur:x>foo <cur:x>bar", Path::new("Foo.java"));
    }
}
