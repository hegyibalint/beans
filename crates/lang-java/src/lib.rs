mod diagnostics;
mod model;
mod parser;
mod projection;
mod resolution;

use beans_core::analysis::FileAnalysis;
use beans_core::language::{
    Language, LanguageAnalysis, LanguageNavigation, LanguageProcessing, NavigationTarget,
};
use beans_core::storage::Revision;
use beans_core::storage::RevisionedStorage;
use beans_platform_jvm::PlatformJvm;
use beans_platform_jvm::model::JvmSource;

use crate::diagnostics::dummy_diagnostic;
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

    pub(crate) fn file_models_at(
        &self,
        revision: Revision,
    ) -> impl Iterator<Item = (&JvmSource, &JavaFile)> {
        self.file_models.iter_at(revision)
    }
}

impl LanguageProcessing<JvmSource, PlatformJvm> for LanguageJava {
    fn accepts(&self, source: &JvmSource) -> bool {
        match source {
            JvmSource::SourceFile { path } => path.extension().is_some_and(|ext| ext == "java"),
            _ => false,
        }
    }

    fn process(
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
}

impl LanguageAnalysis<JvmSource, PlatformJvm> for LanguageJava {
    fn analyze(
        &self,
        java_source: &JvmSource,
        revision: Revision,
        _platform_jvm: &PlatformJvm,
    ) -> Option<FileAnalysis> {
        let java_model = self.file_models.get(java_source, revision)?;
        Some(FileAnalysis {
            diagnostics: dummy_diagnostic(java_model),
            actions: vec![],
        })
    }
}

impl LanguageNavigation<JvmSource, PlatformJvm> for LanguageJava {
    fn find_declarations_for(
        &self,
        source: &JvmSource,
        offset: usize,
        revision: Revision,
        _platform_jvm: &PlatformJvm,
    ) -> Vec<NavigationTarget<JvmSource>> {
        let Some(java_model) = self.file_models.get(source, revision) else {
            return Vec::new();
        };
        let Some((_, declaration)) = java_model.closest_declaration(offset) else {
            return Vec::new();
        };
        let Some(span) = declaration.name_span() else {
            return Vec::new();
        };

        vec![NavigationTarget {
            source: source.clone(),
            span,
        }]
    }
}

impl Language<JvmSource, PlatformJvm> for LanguageJava {
    fn analysis(&self) -> Option<&dyn LanguageAnalysis<JvmSource, PlatformJvm>> {
        Some(self)
    }

    fn navigation(&self) -> Option<&dyn LanguageNavigation<JvmSource, PlatformJvm>> {
        Some(self)
    }
}

impl Default for LanguageJava {
    fn default() -> Self {
        Self::new()
    }
}
