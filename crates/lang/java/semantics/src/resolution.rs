mod iterators;
mod lookup;
mod result;

#[cfg(test)]
mod tests;

use beans_core_engine::Revision;
use beans_core_model::source::Source;
use beans_lang_java_model::{
    File,
    nodes::NodeIndex,
    references::{TypeNameComponent, TypeRef},
};

use self::lookup::{lookup_lexical_type, lookup_member_path};
pub use self::result::{JavaTypeCandidate, ResolutionFailure, ResolutionSuccess, TypeCandidate};

pub struct ResolverContext<'a> {
    revision: Revision,
    source: &'a Source,
    file: &'a File,
    node_index: NodeIndex,
    type_ref: &'a TypeRef,
}

impl<'a> ResolverContext<'a> {
    pub fn new(
        revision: Revision,
        source: &'a Source,
        file: &'a File,
        node_index: NodeIndex,
        type_ref: &'a TypeRef,
    ) -> Self {
        Self {
            revision,
            source,
            file,
            node_index,
            type_ref,
        }
    }
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

        if let Some(result) = find_in_parent_scopes(ctx, segments)? {
            return Ok(result);
        }

        Err(ResolutionFailure::NotFound)
    }
}

fn find_in_parent_scopes(
    ctx: &ResolverContext<'_>,
    segments: &[TypeNameComponent],
) -> Result<Option<ResolutionSuccess>, ResolutionFailure> {
    let (first, remaining) = segments.split_first().expect("segments must not be empty");

    let candidate = match lookup_lexical_type(ctx, first)? {
        None => return Ok(None),
        Some(ResolutionSuccess::Single(candidate)) => candidate,
        Some(ambiguous) => return Ok(Some(ambiguous)),
    };

    lookup_member_path(ctx, candidate, remaining).map(Some)
}
