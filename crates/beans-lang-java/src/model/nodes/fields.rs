use beans_core::ranges::Spanned;
use serde::{Deserialize, Serialize};

use crate::model::references::TypeRef;

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldDeclaration {
    pub name: String,
    pub declared_type: Spanned<TypeRef>,
}
