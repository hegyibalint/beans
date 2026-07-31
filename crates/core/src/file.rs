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
mod tests;
