use crate::SymbolId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationKind {
    Extends,
    Implements,
    Overrides,
    ProtocolExtends,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Relation {
    pub kind: RelationKind,
    pub target: SymbolId,
}
