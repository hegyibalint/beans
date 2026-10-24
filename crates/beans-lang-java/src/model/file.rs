use beans_core::names::Name;
use serde::{Deserialize, Serialize};

use super::{
    imports::Import,
    nodes::{Node, NodeIndex, NodeKind, types::TypeDeclaration},
};

/// Represents a whole `.java` file.
/// Stored as JSON in `models.model` for Java rows (`model_kind = 1`).
#[derive(Debug, Serialize, Deserialize)]
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

    // Decoding bypasses add_node, so restore its tree invariants before traversal.
    pub(crate) fn validate_nodes(&self) -> Result<(), String> {
        let Some(root) = self.nodes.first() else {
            return Err("missing Java compilation unit".into());
        };
        if root.parent().is_some() || !matches!(root.kind(), NodeKind::CompilationUnit) {
            return Err("invalid Java compilation unit".into());
        }
        let mut linked = vec![false; self.nodes.len()];
        linked[0] = true;
        for entry in self.iter_nodes() {
            if entry.index != Self::ROOT_NODE_ID {
                let parent = entry.node.parent().ok_or("missing Java node parent")?;
                if parent.as_usize() >= entry.index.as_usize()
                    || matches!(entry.node.kind(), NodeKind::CompilationUnit)
                {
                    return Err("invalid Java node parent".into());
                }
            }
            for child in entry.node.iter_children() {
                let Some(node) = self.node(child) else {
                    return Err("invalid Java child index".into());
                };
                if node.parent() != Some(entry.index) || linked[child.as_usize()] {
                    return Err("invalid Java child link".into());
                }
                linked[child.as_usize()] = true;
            }
        }
        if linked.iter().any(|linked| !linked) {
            return Err("unlinked Java node".into());
        }
        Ok(())
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
            if declaration.name() != Some(component.as_str()) {
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
mod tests {
    use super::*;
    use crate::model::nodes::types::Kind;
    use beans_core::ranges::{ByteRange, Spanned};

    #[test]
    fn an_empty_file_has_one_parentless_compilation_unit() {
        let file = File::new();
        let root = file.node(File::ROOT_NODE_ID).unwrap();
        assert!(matches!(root.kind(), NodeKind::CompilationUnit));
        assert_eq!(root.parent(), None);
        assert!(root.iter_children().next().is_none());
        assert_eq!(file.iter_nodes().count(), 1);
        assert!(file.node(NodeIndex::new(1)).is_none());
    }

    #[test]
    fn top_level_nodes_are_direct_children_of_the_compilation_unit() {
        let mut file = File::new();
        let top_level = file.add_node(File::ROOT_NODE_ID, NodeKind::Block);
        let nested = file.add_node(top_level, NodeKind::Block);
        assert!(!file.is_top_level(File::ROOT_NODE_ID));
        assert!(file.is_top_level(top_level));
        assert!(!file.is_top_level(nested));
        assert!(!file.is_top_level(NodeIndex::new(10)));
    }

    #[test]
    fn indices_and_links_survive_arena_growth() {
        let mut file = File::new();
        let first = file.add_node(File::ROOT_NODE_ID, NodeKind::Block);
        for _ in 0..256 {
            file.add_node(first, NodeKind::Block);
        }
        assert_eq!(file.node(first).unwrap().parent(), Some(File::ROOT_NODE_ID));
        assert_eq!(file.iter_children(first).count(), 256);
        for entry in file.iter_children(first) {
            assert_eq!(entry.node.parent(), Some(first));
        }
    }

    #[test]
    fn rejected_insertions_leave_the_tree_unchanged() {
        for (parent, kind) in [
            (NodeIndex::new(10), NodeKind::Block),
            (File::ROOT_NODE_ID, NodeKind::CompilationUnit),
        ] {
            let mut file = File::new();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                file.add_node(parent, kind)
            }));
            assert!(result.is_err());
            assert_eq!(file.iter_nodes().count(), 1);
            assert!(file.iter_children(File::ROOT_NODE_ID).next().is_none());
            assert_eq!(
                file.add_node(File::ROOT_NODE_ID, NodeKind::Block),
                NodeIndex::new(1)
            );
        }
    }

    #[test]
    #[should_panic(expected = "invalid parent node index")]
    fn children_reject_an_unknown_parent() {
        let file = File::new();
        let _ = file.iter_children(NodeIndex::new(10));
    }

    #[test]
    #[should_panic(expected = "invalid node index")]
    fn ancestors_reject_an_unknown_start() {
        let file = File::new();
        let _ = file.iter_ancestors(NodeIndex::new(10)).next();
    }

    #[test]
    fn malformed_package_names_cannot_match_even_an_identical_prefix() {
        let mut file = File::new();
        let mut declaration = TypeDeclaration::new(Kind::Class);
        declaration.name = Some(Spanned::new("Example".into(), ByteRange::new(0, 7)));
        file.add_node(File::ROOT_NODE_ID, NodeKind::Type(declaration));
        file.package_name = ["p", "", "q"].into_iter().map(str::to_owned).collect();
        let name = ["p", "", "q", "Example"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert!(file.find_type(&name).is_empty());
    }
}
