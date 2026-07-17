mod diagnostics;
mod model;
mod parser;
mod projection;
mod resolution;

use beans_core::Revision;
use beans_core::analysis::FileAnalysis;
use beans_core::storage::RevisionedStorage;
use beans_platform_jvm::PlatformJvm;
use beans_platform_jvm::model::JvmSource;

use crate::diagnostics::symbol_diagnostics;
use crate::model::JavaFile;
use crate::parser::JavaParser;
use crate::projection::project_to_jvm;

pub struct LanguageJava {
    parser: JavaParser,
    file_models: RevisionedStorage<JvmSource, JavaFile>,
}

impl LanguageJava {
    pub fn new() -> LanguageJava {
        LanguageJava {
            parser: JavaParser::new(),
            file_models: RevisionedStorage::new(),
        }
    }

    /// Whether this source is Java's to translate.
    pub fn accepts(&self, source: &JvmSource) -> bool {
        match source {
            JvmSource::SourceFile { path } => path.extension().is_some_and(|ext| ext == "java"),
            _ => false,
        }
    }

    pub fn process(
        &mut self,
        java_source: JvmSource,
        revision: Revision,
        platform_jvm: &mut PlatformJvm,
        contents: &str,
    ) {
        let java_model =
            self.file_models
                .put(revision, java_source.clone(), self.parser.parse(contents));
        platform_jvm.register(revision, java_source, project_to_jvm(java_model));
    }

    pub fn analyze(&self, java_source: &JvmSource, revision: Revision) -> Option<FileAnalysis> {
        let java_model = self.file_models.get(java_source, revision)?;
        Some(FileAnalysis {
            diagnostics: symbol_diagnostics(java_model),
            actions: vec![],
        })
    }
}

impl Default for LanguageJava {
    fn default() -> Self {
        Self::new()
    }
}
