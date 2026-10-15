use lsp_types::{Position, SemanticToken, SemanticTokens, SemanticTokensParams};

use super::{Features, source_locations::resource_for_uri};

impl Features {
    pub(crate) fn semantic_tokens(&self, params: &SemanticTokensParams) -> Option<SemanticTokens> {
        let document = self.open_documents.get(params.text_document.uri.as_str())?;
        if document.language_id != "java" {
            return None;
        }
        let mut data = Vec::new();
        let mut previous = Position::new(0, 0);
        for range in self.engine.type_tokens(&resource_for_uri(&document.uri)) {
            let start = document.position(range.start())?;
            let end = document.position(range.end())?;
            // LSP 3.17 #textDocument_semanticTokens: positions are relative to the previous token.
            data.push(SemanticToken {
                delta_line: start.line - previous.line,
                delta_start: if start.line == previous.line {
                    start.character - previous.character
                } else {
                    start.character
                },
                length: end.character - start.character,
                token_type: 0,
                token_modifiers_bitset: 0,
            });
            previous = start;
        }
        Some(SemanticTokens {
            result_id: None,
            data,
        })
    }
}
