use beans_core_model::ranges::Spanned;
use beans_lang_java_model::{
    File,
    references::{TypeNameComponent, TypeRef},
};

use crate::{
    Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate, TypeParameterHandle,
    iterators::local_declaration_handle,
    lookup::{lookup_downward, lookup_upward, lookup_upward_from_type_header},
    result::{classify_candidates, push_unique_candidate},
    validation::{validate_candidate_use, validate_resolution},
};

#[derive(Default)]
pub(super) struct State {
    visiting_type_parameters: Vec<TypeParameterHandle>,
}

pub fn resolve(ctx: &Context<'_>) -> Result<TypeCandidate, ResolutionFailure> {
    resolve_with_state(ctx, &mut State::default())
}

pub fn resolve_supertype(ctx: &Context<'_>) -> Result<TypeCandidate, ResolutionFailure> {
    resolve_supertype_with_state(ctx, &mut State::default())
}

fn resolve_with_state(
    ctx: &Context<'_>,
    state: &mut State,
) -> Result<TypeCandidate, ResolutionFailure> {
    let segments = named_segments(ctx)?;
    let (first, remaining) = segments.split_first().expect("segments must not be empty");

    if let Some(candidate) = lookup_upward(ctx, first.value(), state)? {
        let candidate = resolve_remaining_segments(ctx, candidate, remaining, state)?;
        return validate_resolution(ctx, candidate);
    }

    if let Some(candidate) =
        lookup_downward(ctx, File::ROOT_NODE_ID, std::slice::from_ref(first), state)?
    {
        let candidate = resolve_remaining_segments(ctx, candidate, remaining, state)?;
        return validate_resolution(ctx, candidate);
    }

    Err(ResolutionFailure::NotFound)
}

pub(super) fn resolve_supertype_with_state(
    ctx: &Context<'_>,
    state: &mut State,
) -> Result<TypeCandidate, ResolutionFailure> {
    let segments = named_segments(ctx)?;
    let (first, remaining) = segments.split_first().expect("segments must not be empty");

    if let Some(candidate) = lookup_upward_from_type_header(ctx, first.value(), state)? {
        let candidate = resolve_remaining_segments(ctx, candidate, remaining, state)?;
        return validate_resolution(ctx, candidate);
    }

    if let Some(candidate) =
        lookup_downward(ctx, File::ROOT_NODE_ID, std::slice::from_ref(first), state)?
    {
        let candidate = resolve_remaining_segments(ctx, candidate, remaining, state)?;
        return validate_resolution(ctx, candidate);
    }

    Err(ResolutionFailure::NotFound)
}

fn resolve_remaining_segments(
    ctx: &Context<'_>,
    candidate_segment: TypeCandidate,
    remaining_segments: &[Spanned<TypeNameComponent>],
    state: &mut State,
) -> Result<TypeCandidate, ResolutionFailure> {
    if remaining_segments.is_empty() {
        return Ok(candidate_segment);
    }

    if let TypeCandidate::Java(JavaTypeCandidate::TypeParameter(parameter)) = &candidate_segment {
        validate_candidate_use(ctx, &candidate_segment)?;
        return resolve_through_type_parameter(ctx, parameter, remaining_segments, state);
    }

    let parent = local_declaration_handle(ctx, &candidate_segment)
        .map(|handle| handle.node_index())
        .ok_or(ResolutionFailure::Partial)?;
    lookup_downward(ctx, parent, remaining_segments, state)?.ok_or(ResolutionFailure::Partial)
}

fn resolve_through_type_parameter(
    ctx: &Context<'_>,
    parameter: &TypeParameterHandle,
    remaining: &[Spanned<TypeNameComponent>],
    state: &mut State,
) -> Result<TypeCandidate, ResolutionFailure> {
    if state.visiting_type_parameters.contains(parameter) {
        return Err(ResolutionFailure::CircularTypeParameterBound(
            parameter.clone(),
        ));
    }
    state.visiting_type_parameters.push(parameter.clone());

    let result = (|| {
        let parameter_declaration = parameter
            .parameter(ctx.file)
            .ok_or(ResolutionFailure::Partial)?;
        let mut candidates = Vec::new();

        for bound in parameter_declaration
            .bounds
            .iter()
            .flat_map(|bound| bound.types())
        {
            let TypeRef::Named { segments } = bound.value() else {
                return Err(ResolutionFailure::InvalidTypeRef);
            };
            let substituted = TypeRef::Named {
                segments: segments.iter().chain(remaining).cloned().collect(),
            };
            let bound_ctx = Context::new(
                ctx.revision,
                ctx.source,
                ctx.file,
                parameter.owner().node_index(),
                &substituted,
            );

            match resolve_supertype_with_state(&bound_ctx, state) {
                Ok(candidate) => push_unique_candidate(&mut candidates, candidate),
                Err(ResolutionFailure::NotFound | ResolutionFailure::Partial) => {}
                Err(failure) => return Err(failure),
            }
        }

        classify_candidates(candidates.into_iter())?.ok_or(ResolutionFailure::Partial)
    })();

    state.visiting_type_parameters.pop();
    result
}

fn named_segments<'a>(
    ctx: &'a Context<'_>,
) -> Result<&'a [Spanned<TypeNameComponent>], ResolutionFailure> {
    let TypeRef::Named { segments } = ctx.type_ref else {
        return Err(ResolutionFailure::InvalidTypeRef);
    };
    if segments.is_empty() {
        return Err(ResolutionFailure::InvalidTypeRef);
    }

    Ok(segments)
}
