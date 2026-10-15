use beans_core::location::Location;

use crate::model::nodes::NodeIndex;

/// Identifies a declaration within the caller's database snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeclarationHandle {
    source: Location,
    node_index: NodeIndex,
}

impl DeclarationHandle {
    pub fn new(source: Location, node_index: NodeIndex) -> Self {
        Self { source, node_index }
    }

    pub fn source(&self) -> &Location {
        &self.source
    }

    pub fn node_index(&self) -> NodeIndex {
        self.node_index
    }
}
