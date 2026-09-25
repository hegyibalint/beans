pub struct Template {
    pub content: String,
    pub spans: Vec<Span>,
    pub cursors: Vec<Cursor>,
}

/// A position in fixture source, measured in bytes after removing markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub name: Option<String>,
    pub offset: usize,
}

/// A half-open range in fixture source, measured in bytes after removing markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub name: Option<String>,
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Visits each UTF-8 character boundary inside this span, excluding its end.
    /// `source` is the fixture text after removing markers.
    pub fn positions<'a>(&self, source: &'a str) -> impl Iterator<Item = usize> + 'a {
        let start = self.start;
        source
            .get(start..self.end)
            .expect("span must lie on UTF-8 boundaries within the source")
            .char_indices()
            .map(move |(offset, _)| start + offset)
    }
}

#[cfg(test)]
mod tests {
    use super::Span;

    fn span(start: usize, end: usize) -> Span {
        Span {
            name: None,
            start,
            end,
        }
    }

    #[test]
    fn positions_include_the_start_and_exclude_the_end() {
        assert_eq!(
            span(1, 4).positions("abcdef").collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(span(2, 2).positions("abcdef").collect::<Vec<_>>(), []);
    }

    #[test]
    fn positions_skip_bytes_inside_multibyte_characters() {
        assert_eq!(span(1, 7).positions("a😀éz").collect::<Vec<_>>(), [1, 5]);
    }

    #[test]
    #[should_panic(expected = "UTF-8 boundaries")]
    fn positions_reject_a_span_that_splits_a_character() {
        span(2, 5).positions("a😀z").count();
    }
}
