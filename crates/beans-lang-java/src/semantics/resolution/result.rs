use crate::model::{File, nodes::types::TypeParameter};
use crate::semantics::DeclarationHandle;
use beans_core::{
    location::Location, location_match::LocationMatch, ranges::ByteRange, source::SourceSpan,
};

/// An indexed entity paired with one query-specific reachability path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCandidate {
    pub locator: TypeLocator,
    pub provenance: LocationMatch,
}

impl TypeCandidate {
    pub fn new(locator: TypeLocator, range: ByteRange) -> Self {
        let origin = SourceSpan::new(locator.location().clone(), range);
        Self {
            locator,
            provenance: LocationMatch { origin, via: None },
        }
    }

    /// The indexed entity's location, independent of its query-specific provenance.
    pub fn location(&self) -> &Location {
        self.locator.location()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeLocator {
    Declaration(DeclarationHandle),
    TypeParameter(TypeParameterHandle),
    Jvm(Location),
}

impl TypeLocator {
    pub fn location(&self) -> &Location {
        match self {
            Self::Declaration(handle) => handle.source(),
            Self::TypeParameter(handle) => handle.owner().source(),
            Self::Jvm(location) => location,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParameterHandle {
    owner: DeclarationHandle,
    /// Index into the owner's type parameters.
    index: usize,
}

impl TypeParameterHandle {
    pub(crate) fn new(owner: DeclarationHandle, index: usize) -> Self {
        Self { owner, index }
    }

    pub fn owner(&self) -> &DeclarationHandle {
        &self.owner
    }

    pub fn parameter<'a>(&self, file: &'a File) -> Option<&'a TypeParameter> {
        file.node(self.owner.node_index())?
            .kind()
            .as_type()?
            .type_parameters
            .get(self.index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionFailure {
    Query(String),
    NotFound,
    Partial,
    InvalidTypeRef,
    Ambiguous(Vec<TypeCandidate>),
    IllegalTypeParameterUse(TypeParameterHandle),
    CircularTypeParameterBound(TypeParameterHandle),
}

pub(crate) fn classify_candidates(
    candidates: impl Iterator<Item = TypeCandidate>,
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let mut distinct = Vec::new();
    for candidate in candidates {
        push_unique_candidate(&mut distinct, candidate);
    }
    match distinct.len() {
        0 => Ok(None),
        1 => Ok(distinct.pop()),
        _ => Err(ResolutionFailure::Ambiguous(distinct)),
    }
}

pub(crate) fn push_unique_candidate(candidates: &mut Vec<TypeCandidate>, candidate: TypeCandidate) {
    if !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::nodes::NodeIndex;
    use beans_core::location::LocationEntry;

    #[test]
    fn locators_expose_the_location_of_their_declaration_or_class() {
        let source = Location::file("/source");
        let handle = DeclarationHandle::new(source.clone(), NodeIndex::new(1));
        let declaration = TypeLocator::Declaration(handle.clone());
        let parameter = TypeLocator::TypeParameter(TypeParameterHandle::new(handle, 0));
        let binary =
            Location::file("/artifact").entry(LocationEntry::Container("p/Example.class".into()));
        let class = TypeLocator::Jvm(binary.clone());

        assert_eq!(declaration.location(), &source);
        assert_eq!(parameter.location(), &source);
        assert_eq!(class.location(), &binary);
    }

    #[test]
    fn identical_pairs_do_not_manufacture_ambiguity() {
        let candidate = TypeCandidate::new(
            TypeLocator::Jvm(Location::file("/artifact")),
            ByteRange::new(0, 0),
        );
        assert_eq!(
            classify_candidates([candidate.clone(), candidate.clone()].into_iter()),
            Ok(Some(candidate)),
        );
    }
}
