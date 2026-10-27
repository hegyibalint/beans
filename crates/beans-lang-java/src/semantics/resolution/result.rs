use crate::model::{File, nodes::types::TypeParameter};
use crate::semantics::DeclarationHandle;
use beans_core::location::Location;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParameterHandle {
    owner: DeclarationHandle,
    /// An index pointing into which type parameter we talk about in the type
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeCandidate {
    Java(JavaTypeCandidate),
    Jvm(Location),
}

impl TypeCandidate {
    pub fn location(&self) -> &Location {
        match self {
            Self::Java(JavaTypeCandidate::Declaration(handle)) => handle.source(),
            Self::Java(JavaTypeCandidate::TypeParameter(handle)) => handle.owner().source(),
            Self::Jvm(location) => location,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JavaTypeCandidate {
    Declaration(DeclarationHandle),
    TypeParameter(TypeParameterHandle),
}

pub(crate) fn classify_candidates(
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
    fn candidates_expose_the_location_of_their_declaration_or_class() {
        let source = Location::file("/src/Example.java");
        let handle = DeclarationHandle::new(source.clone(), NodeIndex::new(1));
        let declaration = TypeCandidate::Java(JavaTypeCandidate::Declaration(handle.clone()));
        let parameter = TypeCandidate::Java(JavaTypeCandidate::TypeParameter(
            TypeParameterHandle::new(handle, 0),
        ));
        let binary = Location::file("/library.jar")
            .entry(LocationEntry::Container("p/Example.class".into()));
        let class = TypeCandidate::Jvm(binary.clone());

        assert_eq!(declaration.location(), &source);
        assert_eq!(parameter.location(), &source);
        assert_eq!(class.location(), &binary);
    }
}
