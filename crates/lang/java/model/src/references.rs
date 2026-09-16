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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundKind {
    Exact,
    Extends,
    Super,
    Unbounded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeBound<T = TypeRef> {
    kind: BoundKind,
    types: Vec<T>,
}

impl<T> TypeBound<T> {
    pub fn new_exact(primary: T) -> Self {
        Self {
            kind: BoundKind::Exact,
            types: vec![primary],
        }
    }

    /// JLS §4.4 permits additional bounds; wildcard bounds have only one type (§4.5.1).
    pub fn new_extends(primary: T, additional: Vec<T>) -> Self {
        Self {
            kind: BoundKind::Extends,
            types: std::iter::once(primary).chain(additional).collect(),
        }
    }

    pub fn new_super(primary: T) -> Self {
        Self {
            kind: BoundKind::Super,
            types: vec![primary],
        }
    }

    pub fn new_unbounded() -> Self {
        Self {
            kind: BoundKind::Unbounded,
            types: Vec::new(),
        }
    }

    pub fn kind(&self) -> BoundKind {
        self.kind
    }

    pub fn primary(&self) -> Option<&T> {
        self.types.first()
    }

    pub fn additional(&self) -> &[T] {
        self.types.split_first().map_or(&[], |(_, tail)| tail)
    }

    pub fn types(&self) -> &[T] {
        &self.types
    }

    pub fn map_ref<U>(&self, f: impl FnMut(&T) -> U) -> TypeBound<U> {
        TypeBound {
            kind: self.kind,
            types: self.types.iter().map(f).collect(),
        }
    }
}

#[cfg(test)]
mod tests;
