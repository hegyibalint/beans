use beans_core::{engine::Revision, model::source::Source};

use crate::model::nodes::NodeIndex;

/// An address in this vertical's storage, not a pinned snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeclarationHandle {
    revision: Revision,
    source: Source,
    node_index: NodeIndex,
}

impl DeclarationHandle {
    pub fn new(revision: Revision, source: Source, node_index: NodeIndex) -> Self {
        Self {
            revision,
            source,
            node_index,
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn node_index(&self) -> NodeIndex {
        self.node_index
    }
}
