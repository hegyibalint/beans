use beans_lang_java_model::references::TypeNameComponent;

use super::{
    JavaTypeCandidate, ResolutionFailure, ResolutionSuccess, ResolverContext, TypeCandidate,
    iterators::{
        iter_declared_member_types, iter_enclosing_types, iter_member_types, iter_type_parameters,
    },
};

pub(super) fn lookup_lexical_type(
    ctx: &ResolverContext<'_>,
    segment: &TypeNameComponent,
) -> Result<Option<ResolutionSuccess>, ResolutionFailure> {
    for enclosing_type in iter_enclosing_types(ctx) {
        if let Some(result) = lookup_declared_member_types(ctx, &enclosing_type, segment) {
            return Ok(Some(result));
        }

        if let Some(result) = lookup_type_parameters(ctx, &enclosing_type, segment) {
            return Ok(Some(result));
        }

        if let Some(result) = lookup_inherited_member_types(ctx, &enclosing_type, segment)? {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

fn lookup_declared_member_types(
    ctx: &ResolverContext<'_>,
    owner: &TypeCandidate,
    segment: &TypeNameComponent,
) -> Option<ResolutionSuccess> {
    let candidates = iter_declared_member_types(ctx, owner)
        .filter(|candidate| candidate_name(ctx, candidate) == Some(segment.name.as_str()));
    classify(candidates)
}

fn lookup_type_parameters(
    ctx: &ResolverContext<'_>,
    owner: &TypeCandidate,
    segment: &TypeNameComponent,
) -> Option<ResolutionSuccess> {
    let candidates = iter_type_parameters(ctx, owner)
        .filter(|candidate| candidate_name(ctx, candidate) == Some(segment.name.as_str()));
    classify(candidates)
}

fn lookup_inherited_member_types(
    _ctx: &ResolverContext<'_>,
    _owner: &TypeCandidate,
    _segment: &TypeNameComponent,
) -> Result<Option<ResolutionSuccess>, ResolutionFailure> {
    Ok(None)
}

pub(super) fn lookup_member_path(
    ctx: &ResolverContext<'_>,
    mut owner: TypeCandidate,
    segments: &[TypeNameComponent],
) -> Result<ResolutionSuccess, ResolutionFailure> {
    for segment in segments {
        let candidates = iter_member_types(ctx, &owner)
            .filter(|candidate| candidate_name(ctx, candidate) == Some(segment.name.as_str()));

        owner = match classify(candidates).ok_or(ResolutionFailure::Partial)? {
            ResolutionSuccess::Single(candidate) => candidate,
            ambiguous @ ResolutionSuccess::Ambiguous(_) => return Ok(ambiguous),
        };
    }

    Ok(ResolutionSuccess::Single(owner))
}

fn candidate_name<'a>(ctx: &'a ResolverContext<'_>, candidate: &TypeCandidate) -> Option<&'a str> {
    let TypeCandidate::Java(candidate) = candidate else {
        return None;
    };

    let (owner, parameter) = match candidate {
        JavaTypeCandidate::Declaration(handle) => (handle, None),
        JavaTypeCandidate::TypeParameter(handle) => (handle.owner(), Some(handle.index())),
    };
    if owner.revision() != ctx.revision || owner.source() != ctx.source {
        return None;
    }

    let declaration = ctx.file.node(owner.node_index())?.kind().as_type()?;
    match parameter {
        Some(index) => declaration
            .type_parameters
            .get(index)
            .map(|parameter| parameter.name.as_str()),
        None => declaration.name.as_deref(),
    }
}

fn classify(mut candidates: impl Iterator<Item = TypeCandidate>) -> Option<ResolutionSuccess> {
    let first = candidates.next()?;
    let Some(second) = candidates.next() else {
        return Some(ResolutionSuccess::Single(first));
    };

    Some(ResolutionSuccess::Ambiguous(
        std::iter::once(first)
            .chain(std::iter::once(second))
            .chain(candidates)
            .collect(),
    ))
}
