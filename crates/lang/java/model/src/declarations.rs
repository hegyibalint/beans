pub mod fields;
pub mod methods;
pub mod types;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclarationIndex(usize);

impl DeclarationIndex {
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub enum Declaration {
    Type(types::TypeDeclaration),
    Field(fields::FieldDeclaration),
    Method(methods::MethodDeclaration),
}
