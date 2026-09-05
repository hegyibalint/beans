use crate::references::TypeRef;

#[derive(Debug)]
pub struct FieldDeclaration {
    pub name: String,
    pub declared_type: TypeRef,
}
