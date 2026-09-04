use crate::references::{self, TypeRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    Public,
    Protected,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Class,
    Enum,
    Record,
    Interface,
    AnnotationInterface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Abstract,
    Static,
    Final,
    Sealed,
    NonSealed,
    Strictfp,
}

/// Represents a single type parameter used in a type declaration.
/// For example, `<A extends org.foo.Bar<String>>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParameter {
    /// The name of the placeholder.
    /// Often used ones are `T`, `A`, etc...
    pub name: String,
    /// What bounds the `name` abides to
    pub bounds: Vec<references::TypeBound>,
}

#[derive(Debug)]
pub struct TypeDeclaration {
    pub name: Option<String>,
    pub type_parameters: Vec<TypeParameter>,
    pub kind: Kind,
    pub declared_superclass: Option<TypeRef>,
    pub declared_superinterfaces: Vec<TypeRef>,

    /// Plural, as nobody stops somebody writing `public public private class A`
    /// By storing multiple ones, we can diagnose and fix these cases
    pub access: Vec<AccessLevel>,
    /// Plural, as nobody stops somebody writing `abstract abstract final class A`
    /// By storing multiple ones, we can diagnose and fix these cases
    pub modifiers: Vec<Modifier>,
}

impl TypeDeclaration {
    pub fn new(kind: Kind) -> Self {
        Self {
            name: None,
            type_parameters: Vec::new(),
            kind,
            declared_superclass: None,
            declared_superinterfaces: Vec::new(),
            access: Vec::new(),
            modifiers: Vec::new(),
        }
    }
}
