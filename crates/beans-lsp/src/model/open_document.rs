use lsp_types::{Position, TextDocumentItem, Uri};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenDocument {
    pub uri: Uri,
    pub language_id: String,
    pub version: i32,
    pub text: String,
}

impl OpenDocument {
    pub fn new(uri: Uri, language_id: String, version: i32, text: String) -> Self {
        Self {
            uri,
            language_id,
            version,
            text,
        }
    }

    /// Converts an LSP position, whose character is a UTF-16 offset, into a byte offset.
    pub fn byte_offset(&self, position: Position) -> Option<usize> {
        let (start, end) = line_bounds(&self.text, position.line)?;
        let line = &self.text[start..end];
        let target = position.character as usize;
        let mut character = 0;

        for (offset, value) in line.char_indices() {
            if character == target {
                return Some(start + offset);
            }
            character += value.len_utf16();
            if character > target {
                return None;
            }
        }

        // LSP positions beyond a line's length clamp to its end.
        Some(end)
    }

    /// Converts a byte offset into an LSP position with a UTF-16 character offset.
    pub fn position(&self, offset: usize) -> Option<Position> {
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return None;
        }

        let bytes = self.text.as_bytes();
        if offset < bytes.len() && offset > 0 && bytes[offset - 1..=offset] == *b"\r\n" {
            return None;
        }

        let before = &self.text[..offset];
        let line = before.bytes().filter(|byte| *byte == b'\n').count();
        let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
        let character = self.text[line_start..offset].encode_utf16().count();

        Some(Position {
            line: line.try_into().ok()?,
            character: character.try_into().ok()?,
        })
    }
}

impl From<TextDocumentItem> for OpenDocument {
    fn from(document: TextDocumentItem) -> Self {
        Self::new(
            document.uri,
            document.language_id,
            document.version,
            document.text,
        )
    }
}

fn line_bounds(text: &str, target: u32) -> Option<(usize, usize)> {
    let mut start = 0;

    for _ in 0..target {
        start += text[start..].find('\n')? + 1;
    }

    let mut end = text[start..]
        .find('\n')
        .map_or(text.len(), |newline| start + newline);
    if end > start && text.as_bytes()[end - 1] == b'\r' {
        end -= 1;
    }
    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(text: &str) -> OpenDocument {
        OpenDocument::new(
            "file:///Example.java".parse().unwrap(),
            "java".into(),
            1,
            text.into(),
        )
    }

    fn position(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn text_document_items_become_open_documents() {
        let item = TextDocumentItem::new(
            "file:///Example.java".parse().unwrap(),
            "java".into(),
            7,
            "class Example {}".into(),
        );

        let document = OpenDocument::from(item);

        assert_eq!(document.uri.as_str(), "file:///Example.java");
        assert_eq!(document.language_id, "java");
        assert_eq!(document.version, 7);
        assert_eq!(document.text, "class Example {}");
    }

    #[test]
    fn positions_and_offsets_are_computed_from_the_text() {
        let document = document("first\na😀b\n");

        assert_eq!(document.byte_offset(position(1, 3)), Some(11));
        assert_eq!(document.position(11), Some(position(1, 3)));
        assert_eq!(document.byte_offset(position(2, 0)), Some(13));
    }

    #[test]
    fn positions_inside_characters_are_rejected() {
        let document = document("a😀b");

        assert_eq!(document.byte_offset(position(0, 2)), None);
        assert_eq!(document.position(2), None);
    }

    #[test]
    fn positions_beyond_a_line_clamp_to_its_end() {
        assert_eq!(document("abc\n").byte_offset(position(0, 99)), Some(3));
    }

    #[test]
    fn carriage_returns_are_not_part_of_crlf_lines() {
        let document = document("ab\r\nc");

        assert_eq!(document.byte_offset(position(0, 2)), Some(2));
        assert_eq!(document.position(3), None);
        assert_eq!(document.position(4), Some(position(1, 0)));
    }

    #[test]
    fn coordinates_outside_the_document_are_rejected() {
        let document = document("abc");

        assert_eq!(document.byte_offset(position(1, 0)), None);
        assert_eq!(document.position(4), None);
    }
}
