use beans_core::{
    ranges::{ByteRange, Spanned},
    resource::ResourceId,
    revision::Revision,
};

use crate::{
    model::{File, nodes::NodeKind, references::TypeRef},
    semantics::resolution::{Context, ResolutionFailure, query::TypeCandidateQuery, resolve},
};

use super::{Diagnostic, DiagnosticSeverity};

pub(super) fn collect(
    file: &File,
    revision: Revision,
    source: &ResourceId,
    definitions: &dyn TypeCandidateQuery,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for entry in file.iter_nodes() {
        let NodeKind::Field(field) = entry.node.kind() else {
            continue;
        };
        let mut references = Vec::new();
        collect_named_references(&field.declared_type, &mut references);
        for reference in references {
            let context = Context::new(revision, source, file, entry.index, reference.value())
                .with_definitions(definitions);
            // Partial resolution can mean missing resolver support, not invalid Java.
            if resolve(&context) != Err(ResolutionFailure::NotFound) {
                continue;
            }
            let TypeRef::Named { segments } = reference.value() else {
                unreachable!();
            };
            let (Some(first), Some(last)) = (segments.first(), segments.last()) else {
                continue;
            };
            let name = segments
                .iter()
                .map(|segment| segment.value().name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            diagnostics.push(Diagnostic {
                range: ByteRange::new(
                    first.range().start(),
                    last.range().start() + last.value().name.len(),
                ),
                severity: DiagnosticSeverity::Error,
                code: "cannot-find-symbol",
                message: format!("cannot find type: {name}"),
            });
        }
    }
    diagnostics
}

fn collect_named_references<'a>(
    reference: &'a Spanned<TypeRef>,
    references: &mut Vec<&'a Spanned<TypeRef>>,
) {
    match reference.value() {
        TypeRef::Named { segments } => {
            references.push(reference);
            for segment in segments {
                for bound in &segment.value().bounds {
                    for argument in bound.types() {
                        collect_named_references(argument, references);
                    }
                }
            }
        }
        TypeRef::Array { element, .. } => collect_named_references(element, references),
        TypeRef::Primitive(_) | TypeRef::Void => {}
    }
}

#[cfg(test)]
mod tests;
