use beans_core::ranges::Spanned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeRef {
    /// `java.util.Map<String, Integer>`
    Named {
        segments: Vec<Spanned<TypeNameComponent>>,
    },
    /// `int`
    Primitive(PrimitiveType),
    /// `String[][]`
    Array {
        element: Box<Spanned<TypeRef>>,
        dimensions: usize,
    },
    /// `void`
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeNameComponent {
    pub name: String,
    /// The type arguments applied to this name component.
    /// Empty when the source has no explicit type arguments.
    pub bounds: Vec<TypeBound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundKind {
    Exact,
    Extends,
    Super,
    Unbounded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeBound<T = Spanned<TypeRef>> {
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
mod tests {
    use super::{BoundKind, TypeBound};

    #[test]
    fn unbounded_has_no_types() {
        let bound = TypeBound::<u8>::new_unbounded();
        assert_eq!(bound.kind(), BoundKind::Unbounded);
        assert_eq!(bound.primary(), None);
        assert!(bound.additional().is_empty());
        assert!(bound.types().is_empty());
    }

    #[test]
    fn exact_has_only_a_primary_type() {
        let bound = TypeBound::new_exact(1);
        assert_eq!(bound.kind(), BoundKind::Exact);
        assert_eq!(bound.primary(), Some(&1));
        assert!(bound.additional().is_empty());
        assert_eq!(bound.types(), &[1]);
    }

    #[test]
    fn super_has_only_a_primary_type() {
        let bound = TypeBound::new_super(1);
        assert_eq!(bound.kind(), BoundKind::Super);
        assert_eq!(bound.primary(), Some(&1));
        assert!(bound.additional().is_empty());
        assert_eq!(bound.types(), &[1]);
    }

    #[test]
    fn extends_preserves_primary_and_zero_one_or_two_additional_types() {
        for additional in [vec![], vec![2], vec![2, 3]] {
            let bound = TypeBound::new_extends(1, additional.clone());
            assert_eq!(bound.kind(), BoundKind::Extends);
            assert_eq!(bound.primary(), Some(&1));
            assert_eq!(bound.additional(), additional);
            assert_eq!(bound.types().len(), additional.len() + 1);
            assert_eq!(bound.types()[0], 1);
            assert_eq!(&bound.types()[1..], additional);
        }
    }

    #[test]
    fn mapping_preserves_kind_and_visits_every_type_once_in_order() {
        for bound in [
            TypeBound::new_unbounded(),
            TypeBound::new_exact(1),
            TypeBound::new_super(1),
            TypeBound::new_extends(1, vec![]),
            TypeBound::new_extends(1, vec![2]),
            TypeBound::new_extends(1, vec![2, 3]),
        ] {
            let mut visited = Vec::new();
            let mapped = bound.map_ref(|value| {
                visited.push(*value);
                value.to_string()
            });
            assert_eq!(visited.as_slice(), bound.types());
            assert_eq!(mapped.kind(), bound.kind());
            assert_eq!(mapped.types().len(), bound.types().len());
            for (original, transformed) in bound.types().iter().zip(mapped.types()) {
                assert_eq!(transformed, &original.to_string());
            }
        }
    }

    #[test]
    fn mapping_unbounded_does_not_call_the_transformation() {
        let mapped: TypeBound<String> = TypeBound::<u8>::new_unbounded()
            .map_ref(|_| panic!("an unbounded wildcard has no reference to transform"));
        assert_eq!(mapped.kind(), BoundKind::Unbounded);
        assert!(mapped.types().is_empty());
    }

    #[test]
    fn mapping_borrows_without_requiring_clone_or_consuming_the_source() {
        struct Value(u8);
        let bound = TypeBound::new_extends(Value(1), vec![Value(2)]);
        let mapped = bound.map_ref(|value| value.0);
        assert_eq!(mapped.types(), &[1, 2]);
        assert_eq!(bound.primary().unwrap().0, 1);
        assert_eq!(bound.additional()[0].0, 2);
    }

    #[test]
    fn mapping_to_collections_does_not_flatten_the_payloads() {
        let bound = TypeBound::new_extends(1, vec![2, 3]);
        let mapped = bound.map_ref(|value| vec![*value, value + 10]);
        assert_eq!(mapped.primary(), Some(&vec![1, 11]));
        assert_eq!(mapped.additional(), &[vec![2, 12], vec![3, 13]]);
        assert_eq!(mapped.types().len(), 3);
    }
}
