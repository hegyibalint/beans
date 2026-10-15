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
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use beans_core::{location::LocationEntry, ranges::ByteRange, source::SourceSpan};
    use beans_testing::template::Template;
    use lsp_types::{Position, Range};

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

    #[test]
    fn locations_preserve_the_open_documents_uri_spelling() {
        let uri: Uri = "file:///src/%54arget.java".parse().unwrap();
        let (_root, mut features) = crate::features::test_features();
        features.open_documents.insert(
            uri.as_str().into(),
            OpenDocument::new(uri.clone(), "java".into(), 1, "class Target {}".into()),
        );

        let location = features
            .location_for_span(SourceSpan::new(
                Location::file("/src/Target.java"),
                ByteRange::new(6, 12),
            ))
            .unwrap();

        assert_eq!(location.uri, uri);
        assert_eq!(
            location.range,
            Range::new(Position::new(0, 6), Position::new(0, 12))
        );
    }

    #[test]
    fn nested_resources_have_no_source_location_yet() {
        let source =
            Location::file("/library.jar").entry(LocationEntry::Container("Target.class".into()));
        let (_root, features) = crate::features::test_features();
        assert!(
            features
                .location_for_span(SourceSpan::new(source, ByteRange::new(0, 1)))
                .is_none()
        );
    }

    #[test]
    fn missing_text_snapshot_is_read_from_disk_for_coordinates() {
        let path = std::env::temp_dir().join(format!(
            "beans-location-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let _cleanup = Cleanup(path.clone());
        let fixture = Template::parse("/* 😀 */ <span>target</span>");
        fs::write(&path, &fixture.content).unwrap();
        let uri = Url::from_file_path(&path).unwrap().to_string();
        let target = &fixture.spans[0];

        let (_root, features) = crate::features::test_features();
        let location = features
            .location_for_span(SourceSpan::new(
                Location::file(path),
                ByteRange::new(target.start, target.end),
            ))
            .unwrap();

        assert_eq!(location.uri.as_str(), uri);
        assert_eq!(
            location.range,
            Range::new(Position::new(0, 9), Position::new(0, 15))
        );
    }
}
