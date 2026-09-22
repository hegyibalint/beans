//! Application-level composition of language and platform engines.

use std::path::PathBuf;

use beans_core_engine::Revision;
use beans_core_model::source::{Source, SourceLocation};
use beans_lang_java_engine::JavaEngine;
use beans_platform_jvm_engine::JvmEngine;

#[derive(Default)]
pub struct Engine {
    revision: Revision,
    java: JavaEngine,
    jvm: JvmEngine,
}

impl Engine {
    pub fn find_declaration(&self, source: &Source, offset: usize) -> Option<SourceLocation> {
        let Source::SourceFile { path } = source else {
            return None;
        };

        if self.java.accept(path) {
            return self.java.find_declaration(self.revision, source, offset);
        }

        None
    }

    pub fn process(&mut self, path: impl Into<PathBuf>, contents: &str) {
        let path = path.into();

        if self.java.accept(&path) {
            let model = self.java.process(contents);
            let source = Source::source_file(path);
            let revision = self.revision.advance();
            self.java.store(revision, model, source);
            return;
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
    use beans_core_engine::Revision;
    use beans_core_model::{classpath::Classpath, source::Source};

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
        let path = "src/example/Example.java";

        engine.process(path, "package example; public class Example {}");

        assert_eq!(engine.revision(), Revision::new(1));
        let source = Source::source_file(path);
        let classpath = Classpath::default();
        let file = engine
            .java()
            .query(engine.revision(), &classpath)
            .file(&source)
            .unwrap();
        assert_eq!(file.package_name.as_slice(), ["example"]);
    }

    #[test]
    fn unsupported_sources_do_not_advance_the_revision() {
        let mut engine = Engine::default();

        engine.process("README.md", "not Java");

        assert_eq!(engine.revision(), Revision::default());
    }
}
