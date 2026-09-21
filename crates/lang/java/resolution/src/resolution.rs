use beans_lang_java_model::references::{TypeNameComponent, TypeRef};

use crate::{
    ResolutionFailure, Resolver, ResolverContext, TypeCandidate,
    lookup::{lookup_in_enclosing_types, lookup_member_path, lookup_supertype_in_enclosing_types},
    validation::validate_resolution,
};

impl Resolver {
    pub fn resolve(ctx: &ResolverContext<'_>) -> Result<TypeCandidate, ResolutionFailure> {
        let (first, remaining) = named_segments(ctx)?
            .split_first()
            .expect("segments must not be empty");
        let candidate =
            lookup_in_enclosing_types(ctx, first)?.ok_or(ResolutionFailure::NotFound)?;

        let resolution = lookup_member_path(ctx, candidate, remaining)?;
        validate_resolution(ctx, resolution)
    }

    pub fn resolve_supertype(
        ctx: &ResolverContext<'_>,
    ) -> Result<TypeCandidate, ResolutionFailure> {
        let (first, remaining) = named_segments(ctx)?
            .split_first()
            .expect("segments must not be empty");
        let candidate =
            lookup_supertype_in_enclosing_types(ctx, first)?.ok_or(ResolutionFailure::NotFound)?;

        let resolution = lookup_member_path(ctx, candidate, remaining)?;
        validate_resolution(ctx, resolution)
    }
}

fn named_segments<'a>(
    ctx: &'a ResolverContext<'_>,
) -> Result<&'a [TypeNameComponent], ResolutionFailure> {
    let TypeRef::Named { segments } = ctx.type_ref else {
        return Err(ResolutionFailure::InvalidTypeRef);
    };
    if segments.is_empty() {
        return Err(ResolutionFailure::InvalidTypeRef);
    }

    Ok(segments)
}
