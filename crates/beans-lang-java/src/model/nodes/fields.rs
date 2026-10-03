use beans_core::ranges::Spanned;

use crate::model::references::TypeRef;

#[derive(Debug)]
pub struct FieldDeclaration {
    pub name: String,
    pub declared_type: Spanned<TypeRef>,
}
