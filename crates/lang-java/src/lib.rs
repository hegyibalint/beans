mod diagnostics;
mod model;
mod parser;
mod projection;
mod resolution;

use beans_core::analysis::FileAnalysis;
use beans_core::language::{Language, LanguageProcessing, NavigationTarget};
use beans_core::model::Span;
use beans_core::storage::Revision;
use beans_core::storage::RevisionedStorage;
use beans_platform_jvm::PlatformJvm;
use beans_platform_jvm::model::JvmSource;

use crate::diagnostics::unresolved_name_diagnostics;
use crate::model::{JavaDeclarationId, JavaFile};
use crate::parser::JavaParser;
use crate::projection::project_to_jvm;
use crate::resolution::resolve_occurrence_at;

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

    pub(crate) fn iter_file_models_at(
        &self,
        revision: Revision,
    ) -> impl Iterator<Item = (&JvmSource, &JavaFile)> {
        self.file_models.iter_at(revision)
    }

    /// A display name for the declaration whose name sits at `span`:
    /// dotted for types (`p.Outer.Inner`), bare otherwise.
    pub fn declaration_label(
        &self,
        source: &JvmSource,
        span: Span,
        revision: Revision,
    ) -> Option<String> {
        let model = self.file_models.get(source, revision)?;
        let (index, _) = model
            .declarations
            .iter()
            .enumerate()
            .find(|(_, declaration)| declaration.name_span() == Some(span))?;
        model.declaration_label(JavaDeclarationId(index))
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

impl Language<JvmSource, PlatformJvm> for LanguageJava {
    fn analyze(
        &self,
        java_source: &JvmSource,
        revision: Revision,
        _platform_jvm: &PlatformJvm,
    ) -> Option<FileAnalysis> {
        let java_model = self.file_models.get(java_source, revision)?;
        Some(FileAnalysis {
            diagnostics: unresolved_name_diagnostics(java_model),
            actions: vec![],
        })
    }

    fn find_declarations_for(
        &self,
        source: &JvmSource,
        offset: usize,
        revision: Revision,
        platform_jvm: &PlatformJvm,
    ) -> Option<Vec<NavigationTarget<JvmSource>>> {
        let java_model = self.file_models.get(source, revision)?;
        Some(resolve_occurrence_at(
            source,
            java_model,
            offset,
            revision,
            platform_jvm,
            self,
        ))
    }
}

impl Default for LanguageJava {
    fn default() -> Self {
        Self::new()
    }
}
