use crate::model::{
    nodes::{NodeIndex, types::AccessLevel},
    references::TypeNameComponent,
};
use beans_core::model::ranges::Spanned;

use super::{
    Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate,
    iterators::{
        iter_declared_member_types, iter_direct_supertype_refs, iter_enclosing_types,
        iter_type_parameters, iter_types_below, local_declaration_handle, local_type_candidate,
    },
    resolve::{State, resolve_supertype_with_state},
    result::{classify_candidates, push_unique_candidate},
};

/// Searches from the occurrence toward enclosing types for the first component.
///
/// Each enclosing type is one lexical tier: declared member types, type parameters, then
/// inherited member types. The caller commits to the first candidate this lookup returns.
pub(super) fn lookup_upward(
    ctx: &Context<'_>,
    component: &TypeNameComponent,
    state: &mut State,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    for owner in iter_enclosing_types(ctx) {
        if let Some(candidate) = lookup_upward_in_type_body(ctx, &owner, component, state)? {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

/// Performs upward lookup for a superclass, superinterface, or type-parameter bound.
///
/// The declaring type's parameters are visible, but its members are not in scope in its header.
/// Enclosing types use normal type-body precedence (JLS §6.3).
pub(super) fn lookup_upward_from_type_header(
    ctx: &Context<'_>,
    segment: &TypeNameComponent,
    state: &mut State,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let mut enclosing_types = iter_enclosing_types(ctx);

    if let Some(owner) = enclosing_types.next()
        && let Some(candidate) = lookup_upward_type_parameters(ctx, &owner, segment)?
    {
        return Ok(Some(candidate));
    }

    for owner in enclosing_types {
        if let Some(candidate) = lookup_upward_in_type_body(ctx, &owner, segment, state)? {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

/// Resolves a nonempty path among the types below `parent`.
///
/// The parent itself is never a candidate. Type parameters are not searched. Directly declared
/// types take precedence over inherited member types; after selecting a component, lookup proceeds
/// only below that selected type.
pub(super) fn lookup_downward(
    ctx: &Context<'_>,
    parent: NodeIndex,
    segments: &[Spanned<TypeNameComponent>],
    state: &mut State,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let Some((first, remaining)) = segments.split_first() else {
        return Err(ResolutionFailure::InvalidTypeRef);
    };

    let first = first.value();
    let declared = iter_types_below(ctx, parent)
        .filter(|candidate| candidate_name(ctx, candidate) == Some(first.name.as_str()));
    let candidate = match classify_candidates(declared)? {
        Some(candidate) => Some(candidate),
        None => {
            let Some(owner) = local_type_candidate(ctx, parent) else {
                return Ok(None);
            };
            let mut visiting = Vec::new();
            classify_candidates(
                lookup_inherited_member_types(ctx, &owner, first, &mut visiting, state).into_iter(),
            )?
        }
    };
    let Some(candidate) = candidate else {
        return Ok(None);
    };

    if remaining.is_empty() {
        return Ok(Some(candidate));
    }

    let child = local_declaration_handle(ctx, &candidate)
        .map(|handle| handle.node_index())
        .ok_or(ResolutionFailure::Partial)?;
    let candidate =
        lookup_downward(ctx, child, remaining, state)?.ok_or(ResolutionFailure::Partial)?;
    Ok(Some(candidate))
}

fn lookup_upward_in_type_body(
    ctx: &Context<'_>,
    owner: &TypeCandidate,
    segment: &TypeNameComponent,
    state: &mut State,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    if let Some(candidate) = lookup_upward_declared_member_types(ctx, owner, segment)? {
        return Ok(Some(candidate));
    }

    if let Some(candidate) = lookup_upward_type_parameters(ctx, owner, segment)? {
        return Ok(Some(candidate));
    }

    let mut visiting = Vec::new();
    classify_candidates(
        lookup_inherited_member_types(ctx, owner, segment, &mut visiting, state).into_iter(),
    )
}

fn lookup_upward_declared_member_types(
    ctx: &Context<'_>,
    owner: &TypeCandidate,
    segment: &TypeNameComponent,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let candidates = iter_declared_member_types(ctx, owner)
        .filter(|candidate| candidate_name(ctx, candidate) == Some(segment.name.as_str()));
    classify_candidates(candidates)
}

fn lookup_upward_type_parameters(
    ctx: &Context<'_>,
    owner: &TypeCandidate,
    segment: &TypeNameComponent,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let candidates = iter_type_parameters(ctx, owner)
        .filter(|candidate| candidate_name(ctx, candidate) == Some(segment.name.as_str()));
    classify_candidates(candidates)
}

fn lookup_inherited_member_types(
    ctx: &Context<'_>,
    owner: &TypeCandidate,
    segment: &TypeNameComponent,
    visiting: &mut Vec<TypeCandidate>,
    state: &mut State,
) -> Vec<TypeCandidate> {
    if visiting.contains(owner) {
        // Circular inheritance is invalid (JLS §8.1.5, §9.1.3). Until lookup can
        // retain branch diagnostics beside candidates, this branch contributes none.
        return Vec::new();
    }
    visiting.push(owner.clone());

    let mut candidates = Vec::new();
    let Some(owner_index) = local_declaration_handle(ctx, owner).map(|handle| handle.node_index())
    else {
        visiting.pop();
        return candidates;
    };

    for type_ref in iter_direct_supertype_refs(ctx, owner) {
        let supertype_ctx = Context::new(ctx.revision, ctx.source, ctx.file, owner_index, type_ref);
        let supertype = match resolve_supertype_with_state(&supertype_ctx, state) {
            Ok(candidate) => candidate,
            Err(_) => continue,
        };

        let declared = iter_declared_member_types(ctx, &supertype)
            .filter(|candidate| candidate_name(ctx, candidate) == Some(segment.name.as_str()))
            .collect::<Vec<_>>();
        let branch = if declared.is_empty() {
            lookup_inherited_member_types(ctx, &supertype, segment, visiting, state)
        } else {
            declared
        };

        for candidate in branch {
            if is_inherited_member_type(ctx, &candidate) {
                push_unique_candidate(&mut candidates, candidate);
            }
        }
    }

    visiting.pop();
    candidates
}

fn is_inherited_member_type(ctx: &Context<'_>, candidate: &TypeCandidate) -> bool {
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = candidate else {
        return false;
    };
    if handle.revision() != ctx.revision || handle.source() != ctx.source {
        return false;
    }

    ctx.file
        .node(handle.node_index())
        .and_then(|node| node.kind().as_type())
        .is_some_and(|declaration| !declaration.access.contains(&AccessLevel::Private))
}

fn candidate_name<'a>(ctx: &'a Context<'_>, candidate: &TypeCandidate) -> Option<&'a str> {
    let TypeCandidate::Java(candidate) = candidate else {
        return None;
    };

    let owner = match candidate {
        JavaTypeCandidate::Declaration(handle) => handle,
        JavaTypeCandidate::TypeParameter(handle) => handle.owner(),
    };
    if owner.revision() != ctx.revision || owner.source() != ctx.source {
        return None;
    }

    match candidate {
        JavaTypeCandidate::Declaration(handle) => {
            ctx.file.node(handle.node_index())?.kind().as_type()?.name()
        }
        JavaTypeCandidate::TypeParameter(handle) => handle
            .parameter(ctx.file)
            .map(|parameter| parameter.name.as_str()),
    }
}
