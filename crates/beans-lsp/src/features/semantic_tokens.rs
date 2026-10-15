use lsp_types::{Position, SemanticToken, SemanticTokens, SemanticTokensParams};

use super::{Features, source_locations::resource_for_uri};

impl Features {
    pub(crate) fn semantic_tokens(
        &self,
        params: &SemanticTokensParams,
    ) -> Result<Option<SemanticTokens>, String> {
        let Some(document) = self.open_documents.get(params.text_document.uri.as_str()) else {
            return Ok(None);
        };
        if document.language_id != "java" {
            return Ok(None);
        }
        let mut data = Vec::new();
        let mut previous = Position::new(0, 0);
        for range in self.engine.type_tokens(&resource_for_uri(&document.uri))? {
            let Some((start, end)) = document
                .position(range.start())
                .zip(document.position(range.end()))
            else {
                return Ok(None);
            };
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
        Ok(Some(SemanticTokens {
            result_id: None,
            data,
        }))
    }
}
