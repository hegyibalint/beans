use crate::model::{File, nodes::types::TypeParameter};
use crate::semantics::DeclarationHandle;
use beans_platform_jvm::engine::query::ClassHandle;

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
    Jvm(ClassHandle),
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
