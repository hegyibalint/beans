use beans_core::model::ranges::Spanned;

use crate::model::references::TypeRef;

#[derive(Debug)]
pub struct FieldDeclaration {
    pub name: String,
    pub declared_type: Spanned<TypeRef>,
}
