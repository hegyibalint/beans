use beans_core_model::names::Name;

use crate::{
    imports::Import,
    nodes::{Node, NodeIndex, NodeKind, types::TypeDeclaration},
};

/// Represents a whole `.java` file.
#[derive(Debug)]
pub struct File {
    /// Package components; empty for the unnamed package (JLS §7.4.2).
    pub package_name: Name,
    pub imports: Vec<Import>,
    nodes: Vec<Node>,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeEntry<'a> {
    pub index: NodeIndex,
    pub node: &'a Node,
}

impl File {
    pub const ROOT_NODE_ID: NodeIndex = NodeIndex::new(0);

    pub fn new() -> Self {
        Self {
            package_name: Name::default(),
            imports: Vec::new(),
            nodes: vec![Node::new(None, NodeKind::CompilationUnit)],
        }
    }

    pub fn node(&self, index: NodeIndex) -> Option<&Node> {
        self.nodes.get(index.as_usize())
    }

    pub fn is_top_level(&self, index: NodeIndex) -> bool {
        self.node(index)
            .is_some_and(|node| node.parent() == Some(Self::ROOT_NODE_ID))
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = NodeEntry<'_>> + '_ {
        self.nodes
            .iter()
            .enumerate()
            .map(|(index, node)| NodeEntry {
                index: NodeIndex::new(index),
                node,
            })
    }

    /// Walks containment ancestors, including the starting node and compilation unit.
    /// Containment alone does not determine visibility (JLS §6.3).
    pub fn iter_ancestors(&self, index: NodeIndex) -> impl Iterator<Item = NodeEntry<'_>> + '_ {
        std::iter::successors(Some(index), |&index| {
            self.node(index).expect("invalid node index").parent()
        })
        .map(move |index| NodeEntry {
            index,
            node: self.node(index).expect("invalid node index"),
        })
    }

    pub fn iter_children(&self, parent: NodeIndex) -> impl Iterator<Item = NodeEntry<'_>> + '_ {
        self.node(parent)
            .expect("invalid parent node index")
            .iter_children()
            .map(move |index| NodeEntry {
                index,
                node: self.node(index).expect("invalid child node index"),
            })
    }

    pub fn add_node(&mut self, parent: NodeIndex, kind: NodeKind) -> NodeIndex {
        assert!(
            parent.as_usize() < self.nodes.len(),
            "invalid parent node index"
        );
        assert!(
            !matches!(kind, NodeKind::CompilationUnit),
            "compilation unit must be the root"
        );

        let index = NodeIndex::new(self.nodes.len());
        self.nodes.push(Node::new(Some(parent), kind));
        self.nodes[parent.as_usize()].add_child(index);
        index
    }

    /// Finds declarations by canonical name (JLS §6.7), retaining duplicate declarations.
    /// Follows declared members only; does not check accessibility or search inherited members.
    pub fn find_type(&self, name: &Name) -> Vec<(NodeIndex, &TypeDeclaration)> {
        if !self.package_name.is_valid() || !name.is_valid() {
            return Vec::new();
        }

        let Some(type_path) = name.as_slice().strip_prefix(self.package_name.as_slice()) else {
            return Vec::new();
        };

        let mut matches = Vec::new();
        self.collect_type_matches(Self::ROOT_NODE_ID, type_path, &mut matches);
        matches
    }

    fn collect_type_matches<'a>(
        &'a self,
        parent: NodeIndex,
        path: &[String],
        matches: &mut Vec<(NodeIndex, &'a TypeDeclaration)>,
    ) {
        let Some((component, remaining)) = path.split_first() else {
            return;
        };

        for entry in self.iter_children(parent) {
            let Some(declaration) = entry.node.kind().as_type() else {
                continue;
            };
            if declaration.name.as_ref() != Some(component) {
                continue;
            }

            if remaining.is_empty() {
                matches.push((entry.index, declaration));
            } else {
                self.collect_type_matches(entry.index, remaining, matches);
            }
        }
    }
}

#[cfg(test)]
mod tests;
