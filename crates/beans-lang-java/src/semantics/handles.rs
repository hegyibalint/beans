use beans_core::{resource::ResourceId, revision::Revision};

use crate::model::nodes::NodeIndex;

/// An address in this vertical's storage, not a pinned snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeclarationHandle {
    revision: Revision,
    source: ResourceId,
    node_index: NodeIndex,
}

impl DeclarationHandle {
    pub fn new(revision: Revision, source: ResourceId, node_index: NodeIndex) -> Self {
        Self {
            revision,
            source,
            node_index,
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn source(&self) -> &ResourceId {
        &self.source
    }

    pub fn node_index(&self) -> NodeIndex {
        self.node_index
    }
}
