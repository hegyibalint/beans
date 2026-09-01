pub mod fields;
pub mod methods;
pub mod types;

pub type DeclarationIndex = usize;

#[derive(Debug)]
pub enum Declaration {
    Type(types::TypeDeclaration),
    Field(fields::FieldDeclaration),
    Method(methods::MethodDeclaration),
}
