//! Application-level composition of language and platform engines.

use std::path::Path;

use beans_core::engine::Revision;
use beans_core::model::source::Source;
use beans_lang_java::engine::JavaEngine;
use beans_platform_jvm::engine::JvmEngine;

pub mod language_features;

use language_features::LanguageFeatures;

#[derive(Default)]
pub struct Engine {
    revision: Revision,
    java: JavaEngine,
    jvm: JvmEngine,
}

impl Engine {
    pub fn language_features(&self) -> LanguageFeatures<'_> {
        LanguageFeatures::new(self)
    }

    /// Ingests an open document under the URI identity provided by the client.
    pub fn process_document(&mut self, uri: &str, language_id: &str, contents: &str) {
        if language_id != "java" {
            return;
        }
        let model = self.java.process(contents);
        let revision = self.revision.advance();
        self.java.store(revision, model, Source::uri(uri));
    }

    pub fn close_document(&mut self, uri: &str) {
        let revision = self.revision.advance();
        self.java.remove(revision, Source::uri(uri));
    }

    /// Whether any vertical can ingest this file.
    pub fn accept(&self, path: &Path) -> bool {
        self.java.accept(path) || self.jvm.accept(path)
    }

    pub fn process(&mut self, uri: &str, contents: &str) {
        if self.java.accept_uri(uri) {
            let model = self.java.process(contents);
            let revision = self.revision.advance();
            self.java.store(revision, model, Source::uri(uri));
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn java(&self) -> &JavaEngine {
        &self.java
    }

    pub fn java_mut(&mut self) -> &mut JavaEngine {
        &mut self.java
    }

    pub fn jvm(&self) -> &JvmEngine {
        &self.jvm
    }

    pub fn jvm_mut(&mut self) -> &mut JvmEngine {
        &mut self.jvm
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Engine;
    use beans_core::engine::{QueryEnvironment, Revision};
    use beans_core::model::{
        classpath::Unrestricted,
        lsp::features::hover::HoverRequest,
        ranges::ByteRange,
        source::{Source, SourceSpan},
    };

    fn type_reference_at(engine: &Engine, source: &Source, offset: usize) -> Option<ByteRange> {
        engine
            .java()
            .file(engine.revision(), source)?
            .type_reference_at(offset)
    }

    #[test]
    fn the_default_engine_starts_at_the_default_revision() {
        let engine = Engine::default();

        assert_eq!(engine.revision(), Revision::default());
        let _ = engine.java();
        let _ = engine.jvm();
    }

    #[test]
    fn accepts_only_files_a_vertical_can_ingest() {
        let engine = Engine::default();

        assert!(engine.accept(Path::new("src/Example.java")));
        assert!(!engine.accept(Path::new("README.md")));
        assert!(!engine.accept(Path::new("lib/Example.class")));
    }

    #[test]
    fn processing_java_source_advances_the_revision_and_stores_its_model() {
        let mut engine = Engine::default();
        let uri = "file:///src/example/Example.java";

        engine.process(uri, "package example; public class Example {}");

        assert_eq!(engine.revision(), Revision::new(1));
        let source = Source::uri(uri);
        let classpath = Unrestricted;
        let file = engine
            .java()
            .query(QueryEnvironment::new(engine.revision(), &classpath))
            .file(&source)
            .unwrap();
        assert_eq!(file.package_name.as_slice(), ["example"]);
    }

    #[test]
    fn virtual_documents_keep_their_identity_and_latest_type_occurrences() {
        let mut engine = Engine::default();
        let uri = "untitled:Example.java";
        let source = Source::uri(uri);
        engine.process_document(uri, "java", "class C extends First {} ");
        assert_eq!(
            type_reference_at(&engine, &source, 16).map(|range| range.len()),
            Some(5)
        );
        assert_eq!(
            type_reference_at(&engine, &Source::uri("file:///Example.java"), 16),
            None
        );

        engine.process_document(uri, "java", "class C extends Second {} ");
        assert_eq!(
            type_reference_at(&engine, &source, 16).map(|range| range.len()),
            Some(6)
        );
        engine.close_document(uri);
        assert_eq!(type_reference_at(&engine, &source, 16), None);
    }

    #[test]
    fn an_imported_type_in_another_stored_file_reaches_navigation() {
        let mut engine = Engine::default();
        let use_uri = "file:///src/p/Use.java";
        let target_uri = "file:///other/q/Target.java";
        let use_text = "package p; import q.Target; class Use { Target field; }";
        let target_text = "package q; public class Target {}";
        engine.process_document(use_uri, "java", use_text);
        engine.process_document(target_uri, "java", target_text);
        let name_start = target_text.find("Target").unwrap();

        assert_eq!(
            engine
                .language_features()
                .goto_definition(&Source::uri(use_uri), use_text.rfind("Target").unwrap())
                .map(|definition| definition.target),
            Some(SourceSpan::new(
                Source::uri(target_uri),
                ByteRange::new(name_start, name_start + "Target".len()),
            ))
        );
    }

    #[test]
    fn java_hover_content_reaches_the_application_facade() {
        let mut engine = Engine::default();
        let uri = "untitled:Example.java";
        let contents = "class C extends Target {}";
        let source = Source::uri(uri);
        engine.process_document(uri, "java", contents);

        let info = engine
            .language_features()
            .hover(&HoverRequest {
                source: &source,
                contents,
                offset: 16,
            })
            .unwrap();

        assert_eq!(info.contents(), "Target");
        let range = info.range();
        assert_eq!(&contents[range.start()..range.end()], "Target");
        assert!(
            engine
                .language_features()
                .hover(&HoverRequest {
                    source: &Source::uri("file:///README.md"),
                    contents,
                    offset: 16
                })
                .is_none()
        );
    }

    #[test]
    fn unsupported_sources_do_not_advance_the_revision() {
        let mut engine = Engine::default();

        engine.process("file:///README.md", "not Java");
        engine.process_document("untitled:Example.java", "plaintext", "class C extends T {}");

        assert_eq!(engine.revision(), Revision::default());
        assert_eq!(
            type_reference_at(&engine, &Source::uri("untitled:Example.java"), 16),
            None
        );
    }
}
