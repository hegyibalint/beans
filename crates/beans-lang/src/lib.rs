//! Concrete composition of language and platform verticals, independent of editor protocols.

use std::path::Path;

use beans_core::origin::Origin;
use beans_core::revision::Revision;
use beans_lang_java::engine::JavaEngine;
use beans_platform_jvm::engine::JvmEngine;

pub mod features;

pub use features::{
    DefinitionProvider, DefinitionRequest, HoverProvider, HoverRequest, HoverResponse,
    LanguageFeatures, LanguagesFeatureContext, NavigationResult,
};

/// Owns semantic state and exposes features without exposing its verticals.
///
/// ```compile_fail
/// let languages = beans_lang::Languages::default();
/// let _ = languages.java();
/// ```
#[derive(Default)]
pub struct Languages {
    revision: Revision,
    java: JavaEngine,
    jvm: JvmEngine,
}

impl Languages {
    /// Ingests an open document under the URI identity provided by the client.
    pub fn process_document(&mut self, uri: &str, language_id: &str, contents: &str) {
        if language_id != "java" {
            return;
        }
        let model = self.java.process(contents);
        let revision = self.revision.advance();
        self.java.store(revision, model, Origin::uri(uri));
    }

    pub fn close_document(&mut self, uri: &str) {
        let revision = self.revision.advance();
        self.java.remove(revision, Origin::uri(uri));
    }

    /// Whether any vertical can ingest this file.
    pub fn accept(&self, path: &Path) -> bool {
        self.java.accept(path) || self.jvm.accept(path)
    }

    pub fn process(&mut self, uri: &str, contents: &str) {
        if self.java.accept_uri(uri) {
            let model = self.java.process(contents);
            let revision = self.revision.advance();
            self.java.store(revision, model, Origin::uri(uri));
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Languages;
    use beans_core::{classpath::Unrestricted, origin::Origin, ranges::ByteRange};
    use beans_core::{query_scope::QueryScope, revision::Revision};

    fn type_reference_at(
        languages: &Languages,
        source: &Origin,
        offset: usize,
    ) -> Option<ByteRange> {
        languages
            .java
            .file(languages.revision(), source)?
            .type_reference_at(offset)
    }

    #[test]
    fn the_default_dispatcher_starts_at_the_default_revision() {
        let languages = Languages::default();

        assert_eq!(languages.revision(), Revision::default());
    }

    #[test]
    fn accepts_only_files_a_vertical_can_ingest() {
        let languages = Languages::default();

        assert!(languages.accept(Path::new("src/Example.java")));
        assert!(!languages.accept(Path::new("README.md")));
        assert!(!languages.accept(Path::new("lib/Example.class")));
    }

    #[test]
    fn processing_java_source_advances_the_revision_and_stores_its_model() {
        let mut languages = Languages::default();
        let uri = "file:///src/example/Example.java";

        languages.process(uri, "package example; public class Example {}");

        assert_eq!(languages.revision(), Revision::new(1));
        let source = Origin::uri(uri);
        let classpath = Unrestricted;
        let file = languages
            .java
            .query(QueryScope::new(languages.revision(), &classpath))
            .file(&source)
            .unwrap();
        assert_eq!(file.package_name.as_slice(), ["example"]);
    }

    #[test]
    fn virtual_documents_keep_their_identity_and_latest_type_occurrences() {
        let mut languages = Languages::default();
        let uri = "untitled:Example.java";
        let source = Origin::uri(uri);
        languages.process_document(uri, "java", "class C extends First {} ");
        assert_eq!(
            type_reference_at(&languages, &source, 16),
            Some(ByteRange::new(16, 21))
        );
        assert_eq!(
            type_reference_at(&languages, &Origin::uri("file:///Example.java"), 16),
            None
        );

        languages.process_document(uri, "java", "class C extends Second {} ");
        assert_eq!(
            type_reference_at(&languages, &source, 16),
            Some(ByteRange::new(16, 22))
        );
        languages.close_document(uri);
        assert_eq!(type_reference_at(&languages, &source, 16), None);
    }

    #[test]
    fn unsupported_sources_do_not_advance_the_revision() {
        let mut languages = Languages::default();

        languages.process("file:///README.md", "not Java");
        languages.process_document("untitled:Example.java", "plaintext", "class C extends T {}");

        assert_eq!(languages.revision(), Revision::default());
        assert_eq!(
            type_reference_at(&languages, &Origin::uri("untitled:Example.java"), 16),
            None
        );
    }
}
