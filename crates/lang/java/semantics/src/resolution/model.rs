use beans_lang_java_model::nodes::{NodeIndex, types::TypeDeclaration};

#[derive(Debug, Clone, Copy)]
pub(super) struct TypeCandidate<'a> {
    node_index: NodeIndex,
    declaration: &'a TypeDeclaration,
}

impl<'a> TypeCandidate<'a> {
    pub(super) fn new(node_index: NodeIndex, declaration: &'a TypeDeclaration) -> Self {
        Self {
            node_index,
            declaration,
        }
    }

    pub(super) fn node_index(&self) -> NodeIndex {
        self.node_index
    }

    pub(super) fn declaration(&self) -> &'a TypeDeclaration {
        self.declaration
    }
}
