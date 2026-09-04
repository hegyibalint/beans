/// Represents a dot-separated name broken into its components
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameRef {
    Simple(String),
    Qualified(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    /// `java.util.Map<String, Integer>`
    Named { segments: Vec<TypeNameComponent> },
    /// `int`
    Primitive(PrimitiveType),
    /// `String[][]`
    Array {
        element: Box<TypeRef>,
        dimensions: usize,
    },
    /// `void`
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveType {
    Byte,
    Short,
    Int,
    Long,
    Char,
    Float,
    Double,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeNameComponent {
    pub name: String,
    /// The type arguments applied to this name component.
    /// Empty when the source has no explicit type arguments.
    pub bounds: Vec<TypeBound>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeBound {
    /// `A<B>`
    Exact { primary: TypeRef },
    /// `A<? extends B>` or `class A<T extends B & C>`
    Extends {
        primary: TypeRef,
        additional: Vec<TypeRef>,
    },
    /// `A<? super B>`
    Super { primary: TypeRef },
    /// `A<?>`
    Unbounded,
}
