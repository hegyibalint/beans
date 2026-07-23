use crate::model::{LineColumnPosition, LineColumnSpan, Offset, OffsetSpan};

/// A file's text together with its line map: the sole place byte offsets and
/// LSP line/column translate into one another.
///
/// The text is retained so a UTF-16 column can be counted without the editor's
/// live buffer; that is what lets us range a declaration in a file the editor
/// never opened. Internally everything is byte offsets — line/column exists
/// only at the edge, and only here.
#[derive(Debug, Clone)]
pub struct TextFile {
    /// Byte offset of each line start; always begins with 0.
    line_starts: Vec<usize>,
    contents: Box<str>,
}

impl TextFile {
    pub fn new(text: &str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(
                text.bytes()
                    .enumerate()
                    .filter(|&(_, byte)| byte == b'\n')
                    .map(|(offset, _)| offset + 1),
            )
            .collect();
        Self {
            line_starts,
            contents: text.into(),
        }
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    /// Byte offset → line/column (egress: the coordinate LSP wants).
    pub fn line_column(&self, offset: Offset) -> LineColumnPosition {
        let offset = offset.0.min(self.contents.len());
        // The last line start not past `offset`. line_starts[0] == 0 <= offset,
        // so partition_point is always >= 1 and the subtraction never wraps.
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let line_start = self.line_starts[line];
        let character = self.contents[line_start..offset].encode_utf16().count();
        LineColumnPosition {
            line: line as u32,
            character: character as u32,
        }
    }

    pub fn line_column_span(&self, span: OffsetSpan) -> LineColumnSpan {
        LineColumnSpan {
            start: self.line_column(span.start),
            end: self.line_column(span.end),
        }
    }

    /// Line/column → byte offset (ingress: what the editor hands us on the
    /// wire). `None` if the position points outside the file or lands inside a
    /// character, mirroring how the LSP rejects such coordinates.
    pub fn offset(&self, position: LineColumnPosition) -> Option<Offset> {
        let line = position.line as usize;
        let line_start = *self.line_starts.get(line)?;
        // The line runs to the next line start, dropping its trailing '\n';
        // the final line runs to end of text.
        let mut line_end = self
            .line_starts
            .get(line + 1)
            .map_or(self.contents.len(), |&next| next - 1);
        if line_end > line_start && self.contents.as_bytes()[line_end - 1] == b'\r' {
            line_end -= 1;
        }

        let line_text = &self.contents[line_start..line_end];
        let character = position.character as usize;
        if line_text.is_ascii() {
            return (character <= line_text.len()).then_some(Offset(line_start + character));
        }

        let mut utf16_offset = 0;
        for (byte_offset, value) in line_text.char_indices() {
            if utf16_offset == character {
                return Some(Offset(line_start + byte_offset));
            }
            utf16_offset += value.len_utf16();
            if utf16_offset > character {
                return None;
            }
        }
        (utf16_offset == character).then_some(Offset(line_end))
    }
}

impl Default for TextFile {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_column(line: u32, character: u32) -> LineColumnPosition {
        LineColumnPosition { line, character }
    }

    #[test]
    fn line_starts_follow_every_newline() {
        let file = TextFile::new("a\nbb\nccc");
        assert_eq!(file.line_starts, [0, 2, 5]);
    }

    #[test]
    fn offsets_map_to_line_and_column() {
        let file = TextFile::new("a\nbb\nccc");

        assert_eq!(file.line_column(Offset(0)), line_column(0, 0));
        assert_eq!(file.line_column(Offset(2)), line_column(1, 0));
        assert_eq!(file.line_column(Offset(4)), line_column(1, 2));
        assert_eq!(file.line_column(Offset(5)), line_column(2, 0));
    }

    #[test]
    fn columns_count_utf16_code_units() {
        // `😀` is four UTF-8 bytes but two UTF-16 code units.
        let file = TextFile::new("a😀b");
        // `b` sits at byte 5, three UTF-16 units into the line.
        assert_eq!(file.line_column(Offset(5)), line_column(0, 3));
    }

    #[test]
    fn an_offset_past_the_end_clamps_to_the_last_position() {
        let file = TextFile::new("ab\ncd");
        assert_eq!(file.line_column(Offset(999)), line_column(1, 2));
    }

    #[test]
    fn the_empty_file_answers_the_origin() {
        assert_eq!(TextFile::default().line_column(Offset(0)), line_column(0, 0));
    }

    #[test]
    fn offset_finds_ascii_lines_and_columns() {
        let file = TextFile::new("first\nsecond\n");

        assert_eq!(file.offset(line_column(1, 3)), Some(Offset(9)));
        assert_eq!(file.offset(line_column(2, 0)), Some(Offset(13)));
    }

    #[test]
    fn offset_counts_utf16_code_units() {
        let file = TextFile::new("a😀b");

        assert_eq!(file.offset(line_column(0, 1)), Some(Offset(1)));
        assert_eq!(file.offset(line_column(0, 3)), Some(Offset(5)));
        assert_eq!(file.offset(line_column(0, 4)), Some(Offset(6)));
        // Column 2 lands inside the surrogate pair: no such offset.
        assert_eq!(file.offset(line_column(0, 2)), None);
    }

    #[test]
    fn offset_handles_multibyte_bmp_characters() {
        assert_eq!(TextFile::new("éx").offset(line_column(0, 1)), Some(Offset(2)));
    }

    #[test]
    fn offset_rejects_positions_outside_the_file() {
        assert_eq!(TextFile::new("abc").offset(line_column(0, 4)), None);
        assert_eq!(TextFile::new("abc").offset(line_column(1, 0)), None);
    }

    #[test]
    fn offset_excludes_the_carriage_return_from_crlf_lines() {
        let file = TextFile::new("ab\r\nc");

        assert_eq!(file.offset(line_column(0, 2)), Some(Offset(2)));
        assert_eq!(file.offset(line_column(1, 0)), Some(Offset(4)));
    }

    #[test]
    fn offset_and_line_column_round_trip() {
        let file = TextFile::new("class A {\n    B field;\n}\n");
        let position = line_column(1, 4);
        assert_eq!(file.line_column(file.offset(position).unwrap()), position);
    }
}
