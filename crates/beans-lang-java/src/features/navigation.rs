use beans_core::{
    features::navigation::{DefinitionProvider, DefinitionRequest, NavigationResult},
    location::Location,
    ranges::ByteRange,
    resolution::TypeDefinitionQuery,
    source::SourceSpan,
};

use crate::{
    engine::JavaLanguage,
    model::{File, nodes::NodeKind, references::TypeRef, type_references::find_in_reference},
    query::JavaQuery,
    semantics::resolution::{
        Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate, query::ResolutionQuery,
        resolve,
    },
};

use super::JavaFeatureContext;

impl<'request>
    DefinitionProvider<JavaFeatureContext<'_>, DefinitionRequest<'request>, NavigationResult>
    for JavaLanguage
{
    /// Finds a type definition at a Java source position using the visible Java and JVM types.
    fn goto_definition(
        &self,
        context: &JavaFeatureContext<'_>,
        request: &DefinitionRequest<'request>,
    ) -> Result<Option<NavigationResult>, String> {
        let source = request.source;
        let offset = request.offset;
        let java = JavaQuery::new(context.connection, context.scope);
        let definitions = ResolutionQuery::new(context.connection, context.scope);
        let Some(file) = java.file(source)? else {
            return Ok(None);
        };
        if let Some(range) = type_parameter_declaration_at(&file, offset) {
            return Ok(Some(NavigationResult {
                origin_range: range,
                target: SourceSpan::new(source.clone(), range),
            }));
        }
        let Some((origin_range, candidate)) =
            resolve_field_type_at(&file, source, offset, Some(&definitions))
        else {
            return Ok(None);
        };
        let candidate = match candidate {
            Ok(candidate) => candidate,
            Err(ResolutionFailure::Query(error)) => return Err(error),
            Err(_) => return Ok(None),
        };
        let target = match candidate {
            TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) => {
                java.declaration(&handle)?.and_then(|declaration| {
                    declaration
                        .name
                        .map(|name| SourceSpan::new(handle.source().clone(), name.range()))
                })
            }
            TypeCandidate::Java(JavaTypeCandidate::TypeParameter(handle)) => {
                let owner = handle.owner();
                let file = java.file(owner.source())?;
                file.and_then(|file| {
                    handle.parameter(&file).map(|parameter| {
                        SourceSpan::new(owner.source().clone(), parameter.name.range())
                    })
                })
            }
            TypeCandidate::Jvm(_) => None,
        };
        Ok(target.map(|target| NavigationResult {
            origin_range,
            target,
        }))
    }
}

/// A type parameter declaration already identifies its own definition (JLS §4.4).
fn type_parameter_declaration_at(file: &File, offset: usize) -> Option<ByteRange> {
    file.iter_nodes()
        .filter_map(|entry| entry.node.kind().as_type())
        .flat_map(|declaration| &declaration.type_parameters)
        .map(|parameter| parameter.name.range())
        .find(|range| range.contains(offset) || range.end() == offset)
}

/// A missing occurrence is distinct from an occurrence whose name fails resolution.
pub(super) fn resolve_field_type_at(
    file: &File,
    source: &Location,
    offset: usize,
    definitions: Option<&dyn TypeDefinitionQuery<TypeCandidate>>,
) -> Option<(ByteRange, Result<TypeCandidate, ResolutionFailure>)> {
    // JLS §8.3: a field declares a type separately from its variable name.
    let (index, prefix, origin_range) = file.iter_nodes().find_map(|entry| {
        let NodeKind::Field(field) = entry.node.kind() else {
            return None;
        };
        let reference = &field.declared_type;
        let (prefix, origin_range) = if let Some(occurrence) = find_in_reference(reference, offset)
        {
            let mut selected_reference = occurrence.reference;
            while let TypeRef::Array { element, .. } = selected_reference {
                selected_reference = element.value();
            }
            let TypeRef::Named { segments } = selected_reference else {
                return None;
            };
            let selected = segments
                .iter()
                .position(|segment| segment.range() == occurrence.range)?;
            // JLS §6.5.5.2: resolve through this component, not a later member type.
            (
                TypeRef::Named {
                    segments: segments[..=selected].to_vec(),
                },
                occurrence.range,
            )
        } else {
            // A request just after a raw final name can still select it. Dots
            // between components are not part of either occurrence.
            let TypeRef::Named { segments } = reference.value() else {
                return None;
            };
            let last = segments.last()?;
            if !last.value().bounds.is_empty() || last.range().end() != offset {
                return None;
            }
            (
                TypeRef::Named {
                    segments: segments.clone(),
                },
                last.range(),
            )
        };
        Some((entry.index, prefix, origin_range))
    })?;

    let mut ctx = Context::new(source, file, index, &prefix);
    if let Some(definitions) = definitions {
        ctx = ctx.with_definitions(definitions);
    }
    Some((origin_range, resolve(&ctx)))
}
