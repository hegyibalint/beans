use std::result;

use beans_lang_java_model::{
    File,
    nodes::{NodeIndex, types::TypeParameter},
    references::{TypeNameComponent, TypeRef},
};

use crate::resolution::{
    ResolutionFailure::{NotFound, Partial},
    ResolutionSuccess::Ambiguous,
};

pub struct ResolverContext<'a> {
    file: &'a File,
    node_index: NodeIndex,
    type_ref: &'a TypeRef,
}

pub enum ResolutionSuccess {
    Single,
    Ambiguous,
}

pub enum ResolutionFailure {
    NotFound,
    Partial,
    InvalidTypeRef,
}

pub struct Resolver {}

impl Resolver {
    pub fn resolve(ctx: &ResolverContext<'_>) -> Result<ResolutionSuccess, ResolutionFailure> {
        let TypeRef::Named { segments } = ctx.type_ref else {
            return Err(ResolutionFailure::InvalidTypeRef);
        };
        if segments.is_empty() {
            return Err(ResolutionFailure::InvalidTypeRef);
        }

        if let Some(result) = find_in_parent_scopes(ctx.file, ctx.node_index, segments.as_slice())?
        {
            return Ok(result);
        }

        Err(ResolutionFailure::NotFound)
    }
}

fn find_in_parent_scopes(
    file: &File,
    node_index: NodeIndex,
    segments: &[TypeNameComponent],
) -> Result<Option<ResolutionSuccess>, ResolutionFailure> {
    let (first_segment, remaining_segments) = segments.split_first().unwrap();

    let scoped_declarations = file
        .iter_ancestors(node_index)
        .filter_map(|entry| entry.node.kind().as_type());

    for declaration in scoped_declarations {
        let result = find_in_parent_scopes(file, node_index, segments)?;
        if result.is_some() {
            return Ok(result);
        }
    }

    Err(NotFound)
}

fn find_in_members(
    file: &File,
    node_index: NodeIndex,
    segments: &[TypeNameComponent],
) -> Result<Option<ResolutionSuccess>, ResolutionFailure> {
    let (first_segment, remaining_segments) = segments.split_first().unwrap();

    // We find our enclosing type (or return nothing)
    let Some(enclosing_type) = file
        .iter_ancestors(node_index)
        .filter(|entry| entry.node.kind().as_type().is_some())
        .map(|entry| entry.index)
        .next()
    else {
        return Ok(None);
    };

    let mut current_owner = enclosing_type;
    for (depth, segment) in segments.iter().enumerate() {
        let members: Vec<NodeIndex> = file
            .iter_children(current_owner)
            .filter_map(|entry| {
                let Some(declaration) = entry.node.kind().as_type() else {
                    return None;
                };
                let Some(name) = declaration.name.as_ref() else {
                    return None;
                };

                if name != &segment.name {
                    return None;
                } else {
                    Some(entry.index)
                }
            })
            .collect();

        current_owner = match members.as_slice() {
            // We tried with the first segment, we didn't find anything
            // This is fine; what we are looking for is not here
            [] if depth == 1 => return Ok(None),
            // Otherwise, we are already in the chain somewhere
            // If the first segment already matched, this will yield to a compilation error
            [] if depth > 0 => return Err(Partial),
            // We found the matching SINGLE member
            [member] => *member,
            // Otherwise, we have MULTIPLE members
            // We will return them as ambigous
            _ => return Ok(Some(ResolutionSuccess::Ambiguous)),
        };
    }

    Ok(Some(ResolutionSuccess::Single))
}

fn find_in_type_bounds(
    ctx: &ResolverContext<'_>,
    type_parameter: &TypeParameter,
    segments: &[TypeNameComponent],
) -> Result<Option<ResolutionSuccess>, ResolutionFailure> {
    // Here we will look into the bound's types, and see if they have the sub-types matching `segments`
    todo!()
}
