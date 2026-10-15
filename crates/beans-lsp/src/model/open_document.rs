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

    pub fn byte_offset(&self, position: Position) -> Option<usize> {
        let mut start = 0;
        for _ in 0..position.line {
            start += self.text[start..].find('\n')? + 1;
        }
        let mut end = self.text[start..]
            .find('\n')
            .map_or(self.text.len(), |newline| start + newline);
        if end > start && self.text.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        let mut character = 0;
        for (offset, value) in self.text[start..end].char_indices() {
            if character == position.character as usize {
                return Some(start + offset);
            }
            character += value.len_utf16();
            if character > position.character as usize {
                return None;
            }
        }
        Some(end)
    }

    pub fn position(&self, offset: usize) -> Option<Position> {
        Self::position_in(&self.text, offset)
    }

    pub(crate) fn position_in(text: &str, offset: usize) -> Option<Position> {
        if offset > text.len() || !text.is_char_boundary(offset) {
            return None;
        }
        let bytes = text.as_bytes();
        if offset > 0 && offset < bytes.len() && bytes[offset - 1..=offset] == *b"\r\n" {
            return None;
        }
        let before = &text[..offset];
        let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
        Some(Position::new(
            before
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                .try_into()
                .ok()?,
            text[line_start..offset]
                .encode_utf16()
                .count()
                .try_into()
                .ok()?,
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_positions_reject_split_surrogates_and_clamp_past_line_end() {
        let document = OpenDocument::new(
            "untitled:Example".parse().unwrap(),
            "java".into(),
            1,
            "a😀b\r\nc".into(),
        );
        assert_eq!(document.byte_offset(Position::new(0, 2)), None);
        assert_eq!(document.byte_offset(Position::new(0, 3)), Some(5));
        assert_eq!(document.position(5), Some(Position::new(0, 3)));
        assert_eq!(document.byte_offset(Position::new(0, 99)), Some(6));
        assert_eq!(document.position(7), None);
        assert_eq!(document.position(8), Some(Position::new(1, 0)));
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
}
