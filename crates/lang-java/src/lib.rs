mod accessibility;
mod diagnostics;
mod model;
mod parser;
mod projection;
mod query;
mod resolution;

use beans_core::analysis::FileAnalysis;
use beans_core::language::{Language, LanguageProcessing, NavigationTarget};
use beans_core::model::{Offset, OffsetSpan};
use beans_core::storage::Revision;
use beans_core::storage::RevisionedStorage;
use beans_platform_jvm as jvm;

use crate::diagnostics::{access_diagnostics, type_scope_diagnostics, unresolved_name_diagnostics};
use crate::model::{JavaDeclarationId, JavaFile};
use crate::parser::JavaParser;
use crate::projection::project_to_jvm;
use crate::query::JavaQuery;
use crate::resolution::resolve_occurrence_at;

pub struct LanguageJava {
    parser: JavaParser,
    file_models: RevisionedStorage<jvm::model::Source, JavaFile>,
}

impl LanguageJava {
    pub fn new() -> LanguageJava {
        LanguageJava {
            parser: JavaParser::new(),
            file_models: RevisionedStorage::new(),
        }
    }

    /// A display name for the declaration whose name sits at `span`:
    /// dotted for types (`p.Outer.Inner`), bare otherwise.
    pub fn declaration_label(
        &self,
        source: &jvm::model::Source,
        span: OffsetSpan,
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

    pub(crate) fn model_at(
        &self,
        source: &jvm::model::Source,
        revision: Revision,
    ) -> Option<&JavaFile> {
        self.file_models.get(source, revision)
    }
}

impl LanguageProcessing<jvm::model::Source, jvm::Platform> for LanguageJava {
    fn accepts(&self, source: &jvm::model::Source) -> bool {
        match source {
            jvm::model::Source::SourceFile { path } => {
                path.extension().is_some_and(|ext| ext == "java")
            }
            _ => false,
        }
    }

    fn process(
        &mut self,
        java_source: jvm::model::Source,
        revision: Revision,
        platform_jvm: &mut jvm::Platform,
        contents: &str,
    ) {
        // Parse the source file
        let java_model = self.parser.parse(contents);

        // Store it and get the reference to the stored model back
        let java_model = self
            .file_models
            .put(revision, java_source.clone(), java_model);

        // Project the Java model into the interoperable JVM model and register it with the platform
        let jvm_model = project_to_jvm(java_model);
        platform_jvm.register(revision, java_source, jvm_model);
    }
}

impl Language<jvm::model::Source, jvm::Platform> for LanguageJava {
    fn analyze(
        &self,
        java_source: &jvm::model::Source,
        revision: Revision,
        platform_jvm: &jvm::Platform,
    ) -> Option<FileAnalysis> {
        let java_model = self.file_models.get(java_source, revision)?;
        let query = JavaQuery::new(platform_jvm.query_from(java_source, revision), self);

        let mut diagnostics = unresolved_name_diagnostics(java_model);
        diagnostics.extend(type_scope_diagnostics(java_source, java_model, &query));
        diagnostics.extend(access_diagnostics(java_source, java_model, &query));

        Some(FileAnalysis {
            diagnostics,
            actions: vec![],
        })
    }

    fn find_declarations_for(
        &self,
        source: &jvm::model::Source,
        offset: Offset,
        revision: Revision,
        platform_jvm: &jvm::Platform,
    ) -> Option<Vec<NavigationTarget<jvm::model::Source>>> {
        let java_model = self.file_models.get(source, revision)?;
        let query = JavaQuery::new(platform_jvm.query_from(source, revision), self);
        Some(resolve_occurrence_at(source, java_model, offset, &query))
    }
}

impl Default for LanguageJava {
    fn default() -> Self {
        Self::new()
    }
}
