pub(super) mod text_document;
pub(super) mod workspace;

use std::collections::HashMap;

use beans_core::model::source::Source;
use beans_engine::Engine;
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind, Range};

use crate::open_document::OpenDocument;

pub(super) fn hover(
    engine: &Engine,
    open_documents: &HashMap<String, OpenDocument>,
    params: HoverParams,
) -> Option<Hover> {
    let position = params.text_document_position_params;
    let document = open_documents.get(position.text_document.uri.as_str())?;
    let offset = document.byte_offset(position.position)?;
    let source = Source::uri(document.uri.as_str());
    let response = engine.hover(&source, &document.text, offset)?;
    let range = response.range();

    Some(Hover {
        // Bare MarkedString text is Markdown, which can hide generic arguments as HTML tags.
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: response.contents().into(),
        }),
        range: Some(Range::new(
            document.position(range.start())?,
            document.position(range.end())?,
        )),
    })
}
