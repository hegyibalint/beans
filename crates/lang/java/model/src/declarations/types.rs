use crate::NameRef;

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

#[derive(Debug)]
pub struct TypeDeclaration {
    pub name: String,
    pub kind: Kind,
    pub extends: NameRef,
    pub implements: Vec<NameRef>,

    /// Plural, as nobody stops somebody writing `public public private class A`
    /// By storing multiple ones, we can diagnose and fix these cases
    pub access: Vec<AccessLevel>,
    /// Plural, as nobody stops somebody writing `abstract abstract final class A`
    /// By storing multiple ones, we can diagnose and fix these cases
    pub modifiers: Vec<Modifier>,
}
