use beans_engine::diagnostics::DiagnosticSeverity;
use lsp_types::{Diagnostic, NumberOrString, PublishDiagnosticsParams, Range};

use super::{Features, source_locations::resource_for_uri};

impl Features {
    pub(crate) fn diagnostics(&self) -> Vec<PublishDiagnosticsParams> {
        let mut documents: Vec<_> = self
            .open_documents
            .values()
            .filter(|document| document.language_id == "java")
            .collect();
        documents.sort_by_key(|document| document.uri.as_str());
        documents
            .into_iter()
            .map(|document| {
                let diagnostics = self
                    .engine
                    .diagnostics(&resource_for_uri(&document.uri))
                    .into_iter()
                    .filter_map(|diagnostic| {
                        Some(Diagnostic {
                            range: Range::new(
                                document.position(diagnostic.range.start())?,
                                document.position(diagnostic.range.end())?,
                            ),
                            severity: Some(match diagnostic.severity {
                                DiagnosticSeverity::Error => lsp_types::DiagnosticSeverity::ERROR,
                            }),
                            code: Some(NumberOrString::String(diagnostic.code.into())),
                            source: Some("beans".into()),
                            message: diagnostic.message,
                            ..Diagnostic::default()
                        })
                    })
                    .collect();
                PublishDiagnosticsParams::new(
                    document.uri.clone(),
                    diagnostics,
                    Some(document.version),
                )
            })
            .collect()
    }

    pub(crate) fn revision(&self) -> beans_core::revision::Revision {
        self.engine.revision()
    }
}
