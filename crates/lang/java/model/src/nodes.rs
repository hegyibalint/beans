pub mod fields;
pub mod methods;
pub mod types;

use self::{fields::FieldDeclaration, methods::MethodDeclaration, types::TypeDeclaration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIndex(usize);

impl NodeIndex {
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) const fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub enum NodeKind {
    CompilationUnit,
    Type(TypeDeclaration),
    Field(FieldDeclaration),
    Method(MethodDeclaration),
    Block,
}

#[derive(Debug)]
pub struct Node {
    parent: Option<NodeIndex>,
    children: Vec<NodeIndex>,
    kind: NodeKind,
}

impl Node {
    pub(crate) fn new(parent: Option<NodeIndex>, kind: NodeKind) -> Self {
        Self {
            parent,
            children: Vec::new(),
            kind,
        }
    }

    pub fn parent(&self) -> Option<NodeIndex> {
        self.parent
    }

    pub fn iter_children(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.children.iter().copied()
    }

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub(crate) fn add_child(&mut self, child: NodeIndex) {
        self.children.push(child);
    }
}
