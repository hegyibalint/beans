use beans_core::{
    resource::{ResourceId, ResourceRoot},
    revision::Revision,
    source::SourceSpan,
};
use lsp_types::{Location, Range, Uri};
use url::Url;

use super::Features;
use crate::model::open_document::OpenDocument;

impl Features {
    pub(crate) fn location_for_span(
        &self,
        span: SourceSpan,
        revision: Revision,
    ) -> Result<Option<Location>, String> {
        if !span.resource.entries.is_empty() {
            return Ok(None);
        }
        let Some(engine) = &self.engine else {
            return Err("Engine is not initialized".into());
        };
        let Some(text) = engine.source_content(&span.resource, revision)? else {
            return Ok(None);
        };
        let uri = self
            .open_documents
            .values()
            .find(|document| resource_for_uri(&document.uri) == span.resource)
            .map(|document| document.uri.clone());
        let uri = match uri {
            Some(uri) => uri,
            None => match &span.resource.root {
                ResourceRoot::File { path } => Url::from_file_path(path)
                    .ok()
                    .and_then(|uri| uri.as_str().parse().ok()),
                ResourceRoot::Virtual { provider, key } if provider == "lsp" => key.parse().ok(),
                _ => None,
            }
            .ok_or("cannot convert declaration resource to an LSP URI")?,
        };
        let Some((start, end)) = OpenDocument::position_in(&text, span.range.start())
            .zip(OpenDocument::position_in(&text, span.range.end()))
        else {
            return Ok(None);
        };
        Ok(Some(Location::new(uri, Range::new(start, end))))
    }
}

pub(super) fn resource_for_uri(uri: &Uri) -> ResourceId {
    if let Some(path) = Url::parse(uri.as_str())
        .ok()
        .filter(|uri| uri.scheme() == "file")
        .and_then(|uri| uri.to_file_path().ok())
    {
        return ResourceId::file(path);
    }
    ResourceId::new(ResourceRoot::Virtual {
        provider: "lsp".into(),
        key: uri.as_str().into(),
    })
}
