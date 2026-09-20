use beans_platform_jvm_semantics::query::ClassHandle;

use crate::query::{DeclarationHandle, TypeParameterHandle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionSuccess {
    Single(TypeCandidate),
    Ambiguous(Vec<TypeCandidate>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionFailure {
    NotFound,
    Partial,
    InvalidTypeRef,
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
