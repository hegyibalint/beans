use beans_core::ranges::Spanned;
use serde::Serialize;

use crate::model::references::TypeRef;

#[derive(Debug, Serialize)]
pub struct FieldDeclaration {
    pub name: String,
    pub declared_type: Spanned<TypeRef>,
}
