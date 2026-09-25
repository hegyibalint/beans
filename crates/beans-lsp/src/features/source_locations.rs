use std::{borrow::Cow, fs};

use beans_core::model::source::{Source, SourceSpan};
use lsp_types::{Location, Range, Uri};
use url::Url;

use super::Features;
use crate::model::open_document::OpenDocument;

impl Features {
    pub(crate) fn location_for_span(&self, span: SourceSpan) -> Option<Location> {
        let Source::Source { uri } = span.source else {
            return None;
        };
        let document_uri: Uri = uri.parse().ok()?;
        let text: Cow<'_, str> = match self
            .open_documents
            .get(&uri)
            .or_else(|| self.workspace_documents.get(&uri))
        {
            Some(document) => Cow::Borrowed(&document.text),
            None => {
                let path = Url::parse(&uri).ok()?.to_file_path().ok()?;
                Cow::Owned(fs::read_to_string(path).ok()?)
            }
        };

        Some(Location::new(
            document_uri,
            Range::new(
                OpenDocument::position_in(&text, span.range.start())?,
                OpenDocument::position_in(&text, span.range.end())?,
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use beans_core::model::{ranges::ByteRange, source::SourceSpan};
    use beans_testing::template::Template;
    use lsp_types::{Position, Range};

    use super::*;

    #[test]
    fn missing_text_snapshot_is_read_from_disk_for_coordinates() {
        let path = std::env::temp_dir().join(format!(
            "beans-location-{}-{}.java",
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
        let fixture = Template::parse("/* 😀 */ class <span>Widget</span> {}");
        fs::write(&path, &fixture.content).unwrap();
        let uri = Url::from_file_path(&path).unwrap().to_string();
        let target = &fixture.spans[0];

        let location = Features::default()
            .location_for_span(SourceSpan::new(
                Source::uri(&uri),
                ByteRange::new(target.start, target.end),
            ))
            .unwrap();

        assert_eq!(location.uri.as_str(), uri);
        assert_eq!(
            location.range,
            Range::new(Position::new(0, 15), Position::new(0, 21))
        );
    }
}
