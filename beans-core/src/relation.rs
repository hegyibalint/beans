use crate::{SymbolId, TypeRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationKind {
    Extends,
    Implements,
    Overrides,
    Permits,
    ProtocolExtends,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    pub kind: RelationKind,
    pub target: SymbolId,
    /// Type arguments for parameterized supertypes.
    /// e.g., `extends Producer<String>` stores `[TypeRef::Simple("String")]`
    pub type_args: Vec<TypeRef>,
}
