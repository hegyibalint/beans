use std::{borrow::Cow, fs};

use beans_core::{
    location::{Location, LocationEntry},
    source::SourceSpan,
};
use lsp_types::{Location as LspSourceSpan, Range, Uri};
use url::Url;

use super::Features;
use crate::model::open_document::OpenDocument;

impl Features {
    pub(crate) fn location_for_span(&self, span: SourceSpan) -> Option<LspSourceSpan> {
        if span.resource.entries.len() != 1 {
            return None;
        }
        let document = self
            .open_documents
            .values()
            .chain(self.workspace_documents.values())
            .find(|document| resource_for_uri(&document.uri) == span.resource);
        let (uri, text) = if let Some(document) = document {
            (document.uri.clone(), Cow::Borrowed(document.text.as_str()))
        } else {
            let Some(LocationEntry::File(path)) = span.resource.entries.first() else {
                return None;
            };
            let uri = Url::from_file_path(path).ok()?.as_str().parse().ok()?;
            (uri, Cow::Owned(fs::read_to_string(path).ok()?))
        };

        Some(LspSourceSpan::new(
            uri,
            Range::new(
                OpenDocument::position_in(&text, span.range.start())?,
                OpenDocument::position_in(&text, span.range.end())?,
            ),
        ))
    }
}

pub(super) fn resource_for_uri(uri: &Uri) -> Location {
    if let Some(path) = Url::parse(uri.as_str())
        .ok()
        .filter(|uri| uri.scheme() == "file")
        .and_then(|uri| uri.to_file_path().ok())
    {
        return Location::file(path);
    }
    Location::new(LocationEntry::Virtual(uri.as_str().into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_become_file_resources() {
        let uri = "file:///src/My%20Type.java".parse().unwrap();
        assert_eq!(resource_for_uri(&uri), Location::file("/src/My Type.java"));
    }

    #[test]
    fn virtual_uris_keep_the_clients_spelling() {
        let uri: Uri = "BEANS-jvm:///library/Use.class".parse().unwrap();
        assert_eq!(
            resource_for_uri(&uri),
            Location::new(LocationEntry::Virtual(uri.as_str().into()))
        );
    }
}
