use beans_lang_java_model::{nodes::types::AccessLevel, references::TypeNameComponent};

use super::{
    JavaTypeCandidate, ResolutionFailure, Resolver, ResolverContext, TypeCandidate,
    iterators::{
        iter_declared_member_types, iter_direct_supertype_refs, iter_enclosing_types,
        iter_type_parameters, local_declaration_handle,
    },
};

pub(super) fn lookup_in_enclosing_types(
    ctx: &ResolverContext<'_>,
    segment: &TypeNameComponent,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    for enclosing_type in iter_enclosing_types(ctx) {
        if let Some(result) = lookup_in_enclosing_type(ctx, &enclosing_type, segment)? {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

fn lookup_in_enclosing_type(
    ctx: &ResolverContext<'_>,
    enclosing_type: &TypeCandidate,
    segment: &TypeNameComponent,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    if let Some(result) = lookup_declared_member_types(ctx, enclosing_type, segment)? {
        return Ok(Some(result));
    }

    if let Some(result) = lookup_type_parameters(ctx, enclosing_type, segment)? {
        return Ok(Some(result));
    }

    let mut visiting = Vec::new();
    let candidates = lookup_inherited_member_types(ctx, enclosing_type, segment, &mut visiting);
    classify(candidates.into_iter())
}

pub(super) fn lookup_supertype_in_enclosing_types(
    ctx: &ResolverContext<'_>,
    segment: &TypeNameComponent,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let mut enclosing_types = iter_enclosing_types(ctx);

    if let Some(owner) = enclosing_types.next()
        && let Some(result) = lookup_type_parameters(ctx, &owner, segment)?
    {
        return Ok(Some(result));
    }

    // A member's scope is the type body, while type parameters are also in scope
    // in superclass and superinterface clauses (JLS §6.3).
    for enclosing_type in enclosing_types {
        if let Some(result) = lookup_in_enclosing_type(ctx, &enclosing_type, segment)? {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

fn lookup_declared_member_types(
    ctx: &ResolverContext<'_>,
    owner: &TypeCandidate,
    segment: &TypeNameComponent,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let candidates = iter_declared_member_types(ctx, owner)
        .filter(|candidate| candidate_name(ctx, candidate) == Some(segment.name.as_str()));
    classify(candidates)
}

fn lookup_type_parameters(
    ctx: &ResolverContext<'_>,
    owner: &TypeCandidate,
    segment: &TypeNameComponent,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let candidates = iter_type_parameters(ctx, owner)
        .filter(|candidate| candidate_name(ctx, candidate) == Some(segment.name.as_str()));
    classify(candidates)
}

fn lookup_inherited_member_types(
    ctx: &ResolverContext<'_>,
    owner: &TypeCandidate,
    segment: &TypeNameComponent,
    visiting: &mut Vec<TypeCandidate>,
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
        let supertype_ctx =
            ResolverContext::new(ctx.revision, ctx.source, ctx.file, owner_index, type_ref);
        let supertypes = match Resolver::resolve_supertype(&supertype_ctx) {
            Ok(candidate) => vec![candidate],
            Err(_) => continue,
        };

        for supertype in supertypes {
            let declared = iter_declared_member_types(ctx, &supertype)
                .filter(|candidate| candidate_name(ctx, candidate) == Some(segment.name.as_str()))
                .collect::<Vec<_>>();

            let branch = if declared.is_empty() {
                lookup_inherited_member_types(ctx, &supertype, segment, visiting)
            } else {
                declared
            };

            for candidate in branch {
                if is_inherited_member_type(ctx, &candidate) && !candidates.contains(&candidate) {
                    candidates.push(candidate);
                }
            }
        }
    }

    visiting.pop();
    candidates
}

fn is_inherited_member_type(ctx: &ResolverContext<'_>, candidate: &TypeCandidate) -> bool {
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

pub(super) fn lookup_member_path(
    ctx: &ResolverContext<'_>,
    mut owner: TypeCandidate,
    segments: &[TypeNameComponent],
) -> Result<TypeCandidate, ResolutionFailure> {
    for segment in segments {
        let result = match lookup_declared_member_types(ctx, &owner, segment)? {
            Some(result) => Some(result),
            None => {
                let mut visiting = Vec::new();
                let inherited = lookup_inherited_member_types(ctx, &owner, segment, &mut visiting);
                classify(inherited.into_iter())?
            }
        };

        owner = result.ok_or(ResolutionFailure::Partial)?;
    }

    Ok(owner)
}

fn candidate_name<'a>(ctx: &'a ResolverContext<'_>, candidate: &TypeCandidate) -> Option<&'a str> {
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
        JavaTypeCandidate::Declaration(handle) => ctx
            .file
            .node(handle.node_index())?
            .kind()
            .as_type()?
            .name
            .as_deref(),
        JavaTypeCandidate::TypeParameter(handle) => handle
            .parameter(ctx.file)
            .map(|parameter| parameter.name.as_str()),
    }
}

fn classify(
    mut candidates: impl Iterator<Item = TypeCandidate>,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let Some(first) = candidates.next() else {
        return Ok(None);
    };
    let Some(second) = candidates.next() else {
        return Ok(Some(first));
    };

    Err(ResolutionFailure::Ambiguous(
        std::iter::once(first)
            .chain(std::iter::once(second))
            .chain(candidates)
            .collect(),
    ))
}
