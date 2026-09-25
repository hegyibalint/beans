use crate::model::references::TypeNameComponent;
use beans_core::model::ranges::Spanned;

use super::super::{Context, ResolutionFailure, TypeCandidate};

pub(in crate::semantics::resolution) fn lookup_external(
    _ctx: &Context<'_>,
    _segments: &[Spanned<TypeNameComponent>],
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    // TODO: search visible Java and JVM definitions using compilation-unit imports.
    Ok(None)
}
