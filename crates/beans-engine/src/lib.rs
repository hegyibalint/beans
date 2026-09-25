//! Application-level composition of language and platform engines.

use beans_core::engine::Revision;
use beans_core::model::{
    lsp::features::hover::HoverResponse,
    ranges::ByteRange,
    source::{Source, SourceLocation},
};
use beans_lang_java::engine::JavaEngine;
use beans_platform_jvm::engine::JvmEngine;

#[derive(Default)]
pub struct Engine {
    revision: Revision,
    java: JavaEngine,
    jvm: JvmEngine,
}

impl Engine {
    /// Gets language-provided hover content for the modeled source at the current revision.
    pub fn hover(
        &self,
        source: &Source,
        contents: &str,
        offset: usize,
    ) -> Option<Box<dyn HoverResponse>> {
        beans_lang_java::lsp::features::hover(&self.java, self.revision, source, contents, offset)
    }

    /// Finds a modeled Java type identifier, without resolving its declaration.
    pub fn type_reference_at(&self, source: &Source, offset: usize) -> Option<ByteRange> {
        let file = self.java.file(self.revision, source)?;
        beans_lang_java::lsp::type_reference_at(file, offset)
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

    pub fn find_declaration(&self, _source: &Source, _offset: usize) -> Option<SourceLocation> {
        None
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
    use super::Engine;
    use beans_core::engine::Revision;
    use beans_core::model::{classpath::Classpath, source::Source};

    #[test]
    fn the_default_engine_starts_at_the_default_revision() {
        let engine = Engine::default();

        assert_eq!(engine.revision(), Revision::default());
        let _ = engine.java();
        let _ = engine.jvm();
    }

    #[test]
    fn processing_java_source_advances_the_revision_and_stores_its_model() {
        let mut engine = Engine::default();
        let uri = "file:///src/example/Example.java";

        engine.process(uri, "package example; public class Example {}");

        assert_eq!(engine.revision(), Revision::new(1));
        let source = Source::uri(uri);
        let classpath = Classpath::default();
        let file = engine
            .java()
            .query(engine.revision(), &classpath)
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
            engine
                .type_reference_at(&source, 16)
                .map(|range| range.len()),
            Some(5)
        );
        assert_eq!(
            engine.type_reference_at(&Source::uri("file:///Example.java"), 16),
            None
        );

        engine.process_document(uri, "java", "class C extends Second {} ");
        assert_eq!(
            engine
                .type_reference_at(&source, 16)
                .map(|range| range.len()),
            Some(6)
        );
        engine.close_document(uri);
        assert_eq!(engine.type_reference_at(&source, 16), None);
    }

    #[test]
    fn java_hover_content_reaches_the_application_facade() {
        let mut engine = Engine::default();
        let uri = "untitled:Example.java";
        let contents = "class C extends Target {}";
        let source = Source::uri(uri);
        engine.process_document(uri, "java", contents);

        let info = engine.hover(&source, contents, 16).unwrap();

        assert_eq!(info.contents(), "Target");
        let range = info.range();
        assert_eq!(&contents[range.start()..range.end()], "Target");
        assert!(
            engine
                .hover(&Source::uri("file:///README.md"), contents, 16)
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
            engine.type_reference_at(&Source::uri("untitled:Example.java"), 16),
            None
        );
    }
}
