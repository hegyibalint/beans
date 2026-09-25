use beans_core::model::ranges::{ByteRange, Spanned};

use crate::model::{
    File,
    nodes::NodeKind,
    references::{TypeBound, TypeRef},
};

/// A type reference found at a source byte, and the identifier under that byte.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TypeReferenceOccurrence<'a> {
    pub(crate) reference: &'a TypeRef,
    pub(crate) range: ByteRange,
    pub(crate) type_range: ByteRange,
}

/// Only modeled type positions are searched; this does not resolve the reference.
pub(crate) fn type_reference_at(file: &File, offset: usize) -> Option<TypeReferenceOccurrence<'_>> {
    file.iter_nodes().find_map(|entry| match entry.node.kind() {
        NodeKind::Type(declaration) => declaration
            .type_parameters
            .iter()
            .flat_map(|parameter| &parameter.bounds)
            .find_map(|bound| find_in_bound(bound, offset))
            .or_else(|| {
                declaration
                    .declared_superclass
                    .as_ref()
                    .and_then(|reference| find_in_reference(reference, offset))
            })
            .or_else(|| {
                declaration
                    .declared_superinterfaces
                    .iter()
                    .find_map(|reference| find_in_reference(reference, offset))
            }),
        NodeKind::Field(declaration) => find_in_reference(&declaration.declared_type, offset),
        _ => None,
    })
}

fn find_in_bound(bound: &TypeBound, offset: usize) -> Option<TypeReferenceOccurrence<'_>> {
    bound
        .types()
        .iter()
        .find_map(|reference| find_in_reference(reference, offset))
}

fn find_in_reference(
    reference: &Spanned<TypeRef>,
    offset: usize,
) -> Option<TypeReferenceOccurrence<'_>> {
    if !reference.range().contains(offset) {
        return None;
    }

    match reference.value() {
        TypeRef::Named { segments } => {
            // JLS §4.3 and §4.5.1: each qualified identifier may have type arguments;
            // a nested argument is a separate reference, not part of its owner's name.
            segments
                .iter()
                .flat_map(|segment| &segment.value().bounds)
                .find_map(|bound| find_in_bound(bound, offset))
                .or_else(|| {
                    segments.iter().find_map(|segment| {
                        segment
                            .range()
                            .contains(offset)
                            .then_some(TypeReferenceOccurrence {
                                reference: reference.value(),
                                range: segment.range(),
                                type_range: reference.range(),
                            })
                    })
                })
        }
        TypeRef::Array { element, .. } => {
            find_in_reference(element, offset).map(|mut occurrence| {
                // A nested type argument is its own reference, not the enclosing array type.
                if occurrence.type_range == element.range() {
                    occurrence.reference = reference.value();
                    occurrence.type_range = reference.range();
                }
                occurrence
            })
        }
        TypeRef::Primitive(_) | TypeRef::Void => Some(TypeReferenceOccurrence {
            reference: reference.value(),
            range: reference.range(),
            type_range: reference.range(),
        }),
    }
}

#[cfg(test)]
mod tests;
