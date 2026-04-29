use crate::{AnnotationInstance, Location, Modifier, Relation, Signature, SymbolId, SymbolKind};

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub id: SymbolId,
    pub fqn: String,
    pub name: String,
    pub kind: SymbolKind,
    pub location: Option<Location>,
    pub modifiers: Vec<Modifier>,
    pub annotations: Vec<AnnotationInstance>,
    pub parent: Option<SymbolId>,
    pub children: Vec<SymbolId>,
    pub relations: Vec<Relation>,
    pub signature: Option<Signature>,
}
