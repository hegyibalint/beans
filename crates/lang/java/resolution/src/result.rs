use beans_lang_java_model::{File, nodes::types::TypeParameter};
use beans_lang_java_semantics::query::DeclarationHandle;
use beans_platform_jvm_semantics::query::ClassHandle;

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
