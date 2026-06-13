//! Graph arena and `NodeData<P>`.
//!
//! Per ADR-0027 the layer-1 graph is a typed arena with a hard-link
//! forest and RAII handles, and nothing more. Lazy recomputation,
//! push-stale propagation, and stable-vs-volatile lifecycle behavior
//! are layer-2 consumer concerns.
//!
//! Per ADR-0007 `NodeId` is a runtime-only identity. It's an opaque
//! generational handle the engine mints from arena slots — *not*
//! meaningful across rebuilds, snapshots, or any operation that doesn't
//! preserve the arena byte-for-byte.
//!
//! Per ADR-0006 (hard-link half) hard links are stored as `Vec<NodeId>`
//! on the parent. When a node is destroyed, the GC walks its hard-link
//! subtree post-order and frees every descendant; each freed slot bumps
//! its generation so any outstanding `NodeId` pointing at the old
//! occupant gracefully resolves to `None` rather than silently aliasing
//! a recycled neighbour.

/// Marker trait for type-erased RAII handles stored on
/// [`NodeData::handles`]. Per ADR-0014 each handle is its own RAII anchor —
/// its `Drop` impl performs whatever cleanup the handle owns (typically
/// removing a registry entry, but the graph layer doesn't know or care).
/// The trait has no methods; it exists so the engine can type-erase
/// handles without going through `dyn Drop` (which clippy warns against,
/// since `Drop` is special-cased and can be misleading as a trait
/// object).
///
/// The trait lives here, next to its consumer [`NodeData::handles`], so
/// the graph module has no dependency on any specific handle producer
/// (e.g. `crate::registry`). Producers impl this trait for their handle
/// types in their own module.
pub trait NodeHandle {}

/// Opaque, generational arena handle. Pairs a slot index with the
/// generation of that slot at the time the id was minted; the slot's
/// generation bumps every time the slot is freed, so a stale id no
/// longer matches its slot's current occupant. [`Graph::get`] returns
/// `None` on a generation mismatch — consumers never observe a "wrong
/// node at this id" outcome.
///
/// `Copy + Eq + Hash`. Stable across registry mutations (the registry
/// stores `NodeId`s as values, not borrows). Not stable across rebuilds
/// or snapshot reload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId {
    pub(crate) slot: u32,
    pub(crate) generation: u32,
}

impl NodeId {
    /// Construct a `NodeId` for use as a placeholder in tests inside
    /// `beans-core`. Not exposed to external consumers — they receive
    /// `NodeId`s only by inserting into a [`Graph`].
    #[cfg(test)]
    pub(crate) fn placeholder(slot: u32) -> Self {
        Self { slot, generation: 0 }
    }
}

/// Per-slot arena cell. Tracks the slot's *current* generation alongside
/// its (possibly absent) data; generation persists across `Some → None →
/// Some` cycles so reused slots are observably distinct from their
/// previous occupants.
struct Slot<P> {
    /// Bumps every time this slot is freed. A `NodeId` matches this slot
    /// only when its `generation` field equals this value.
    generation: u32,
    data: Option<NodeData<P>>,
}

impl<P: std::fmt::Debug> std::fmt::Debug for Slot<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slot")
            .field("generation", &self.generation)
            .field("data", &self.data)
            .finish()
    }
}

/// Per-node storage. The payload `P` is the union of all node variants
/// the engine cares about (defined by the consumer; this module is generic).
///
/// Per ADR-0014, RAII registration handles live on `NodeData` itself:
/// when the slot is freed, [`handles`](Self::handles) drops, each handle's
/// `Drop` runs, and registry entries are removed automatically. The
/// engine doesn't know the registry types involved — handles are stored
/// as `Box<dyn NodeHandle>` and unregistration is a side effect of dropping
/// them in `Vec`-drop order.
///
/// Keeping handles on the node (not on the payload) lets the *payload*
/// stay free of `Rc`-flavoured `!Send` types, which is what makes
/// pre-integration parse output (e.g. `ParsedJavaFile`) safe to compute
/// on a rayon worker per ADR-0005. The integrated node stays
/// thread-local per ADR-0018, but its payload value can travel.
pub struct NodeData<P> {
    pub payload: P,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    /// RAII handles installed by `NodeBehavior::on_created` after the node
    /// is in the arena. Stored as `Box<dyn NodeHandle>` because the engine
    /// has no per-key knowledge; each impl drops itself.
    pub handles: Vec<Box<dyn NodeHandle>>,
}

// `Vec<Box<dyn NodeHandle>>` doesn't impl `Debug`, so we manually derive a
// `Debug` that elides the handles. They're opaque to anything except
// their own Drop impls.
impl<P: std::fmt::Debug> std::fmt::Debug for NodeData<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeData")
            .field("payload", &self.payload)
            .field("parent", &self.parent)
            .field("children", &self.children)
            .field("handles", &format_args!("[{} handles]", self.handles.len()))
            .finish()
    }
}

/// Single-payload graph arena. Owns a flat `Vec<Slot<P>>`; free slots are
/// tracked in a `Vec<usize>` and reused on the next insert with their
/// generation bumped.
///
/// The engine is generic over `P` so the same machinery serves the JVM
/// payload union, test payloads, and any future tagged variant.
pub struct Graph<P> {
    slots: Vec<Slot<P>>,
    free: Vec<usize>,
}

impl<P: std::fmt::Debug> std::fmt::Debug for Graph<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Graph")
            .field("slots", &self.slots)
            .field("free", &self.free)
            .finish()
    }
}

impl<P> Default for Graph<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P> Graph<P> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Allocate a slot and store `payload` in it. If `parent` is set, the
    /// new node is appended to the parent's `children` (a hard link).
    pub fn insert(&mut self, payload: P, parent: Option<NodeId>) -> NodeId {
        let data = NodeData {
            payload,
            parent,
            children: Vec::new(),
            handles: Vec::new(),
        };

        let (slot_index, generation) = match self.free.pop() {
            Some(idx) => {
                debug_assert!(self.slots[idx].data.is_none());
                self.slots[idx].data = Some(data);
                (idx, self.slots[idx].generation)
            }
            None => {
                let idx = self.slots.len();
                self.slots.push(Slot {
                    generation: 0,
                    data: Some(data),
                });
                (idx, 0)
            }
        };

        let id = NodeId {
            slot: slot_index as u32,
            generation,
        };

        if let Some(parent_id) = parent {
            // Parent must exist; treating a missing parent as a programmer error.
            // Index via the same generation-validating get_mut so an out-of-range
            // or stale parent NodeId surfaces through a descriptive expect.
            self.get_mut(parent_id)
                .expect("insert: parent NodeId references an empty or stale slot")
                .children
                .push(id);
        }

        id
    }

    /// Return the node referenced by `id`, or `None` if the slot is empty
    /// or the slot's generation doesn't match. The generation check is
    /// what makes stale ids (held across a destroy) safe to dereference.
    pub fn get(&self, id: NodeId) -> Option<&NodeData<P>> {
        let slot = self.slots.get(id.slot as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.data.as_ref()
    }

    /// Mutable variant of [`get`](Self::get) with the same generation
    /// validation.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut NodeData<P>> {
        let slot = self.slots.get_mut(id.slot as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.data.as_mut()
    }

    /// True if the slot currently holds a node and `id`'s generation
    /// matches the slot's current generation. False after `destroy` *or*
    /// if the slot has been reused with a fresh generation.
    pub fn contains(&self, id: NodeId) -> bool {
        self.get(id).is_some()
    }

    /// Recursive, post-order destroy: every descendant is freed before
    /// the node itself. Per ADR-0014 each [`NodeData`]'s
    /// [`handles`](NodeData::handles) vec drops as the slot is freed,
    /// and each handle's `Drop` removes its registry entry. The slot's
    /// generation bumps on free, so any outstanding `NodeId` pointing
    /// at this node thereafter resolves to `None`.
    ///
    /// If `id` has a parent, it is also detached from the parent's
    /// `children` list. Calling `destroy` on a non-existent or stale
    /// slot is a no-op.
    pub fn destroy(&mut self, id: NodeId) {
        if !self.contains(id) {
            return;
        }

        // Detach from parent first so the parent's children list is correct
        // even if a Drop side-effect inspects the graph mid-walk.
        if let Some(parent_id) = self.get(id).and_then(|n| n.parent)
            && let Some(parent) = self.get_mut(parent_id)
        {
            parent.children.retain(|c| *c != id);
        }

        self.destroy_subtree(id);
    }

    fn destroy_subtree(&mut self, id: NodeId) {
        // Take ownership of the children list so we can recurse without
        // double-borrowing the slot.
        let children = match self.get_mut(id) {
            Some(node) => std::mem::take(&mut node.children),
            None => return,
        };

        for child in children {
            self.destroy_subtree(child);
        }

        // Now drop the node itself; its `handles` vec drops here, and
        // each contained `Box<dyn NodeHandle>` runs its `Drop` impl.
        // Bumping the generation invalidates any outstanding `NodeId`
        // pointing at this slot.
        let slot_idx = id.slot as usize;
        if let Some(slot) = self.slots.get_mut(slot_idx) {
            slot.data = None;
            slot.generation = slot.generation.wrapping_add(1);
            self.free.push(slot_idx);
        }
    }

    /// Iterate over every occupied node in the arena, yielding
    /// `(NodeId, &NodeData<P>)`. Order is by ascending slot index;
    /// freed slots are skipped. The iterator is not invalidated by
    /// reads but is invalidated by mutation, like any borrow over the
    /// arena.
    ///
    /// For inspection, debugging, snapshotting, or test-harness fallback
    /// use. Semantic resolution (cross-file lookup, member resolution,
    /// type lookup, etc.) goes through registries — `iter` is O(n) over
    /// the entire graph and must not appear on hot paths. If you find
    /// yourself reaching for `iter` to answer a real query, you almost
    /// certainly want a dedicated [`Registry`](crate::registry::Registry)
    /// keyed by whatever discriminator the query carries.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &NodeData<P>)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(idx, slot)| {
                slot.data.as_ref().map(|n| {
                    (
                        NodeId {
                            slot: idx as u32,
                            generation: slot.generation,
                        },
                        n,
                    )
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestNode {
        name: &'static str,
    }

    impl TestNode {
        fn new(name: &'static str) -> Self {
            Self { name }
        }
    }

    #[test]
    fn insert_and_lookup() {
        let mut graph: Graph<TestNode> = Graph::new();
        let id = graph.insert(TestNode::new("alpha"), None);

        let node = graph.get(id).expect("node should exist");
        assert_eq!(node.payload.name, "alpha");
        assert_eq!(node.parent, None);
        assert!(node.children.is_empty());

        // NodeId stable within session — two reads return the same slot.
        assert_eq!(graph.get(id).unwrap().payload.name, "alpha");
        assert_eq!(id.slot, 0);
        assert_eq!(id.generation, 0);
    }

    #[test]
    fn hard_link_cascade() {
        let mut graph: Graph<TestNode> = Graph::new();
        let parent = graph.insert(TestNode::new("parent"), None);
        let child = graph.insert(TestNode::new("child"), Some(parent));
        let grandchild = graph.insert(TestNode::new("grandchild"), Some(child));

        assert_eq!(graph.get(parent).unwrap().children, vec![child]);
        assert_eq!(graph.get(child).unwrap().children, vec![grandchild]);
        assert_eq!(graph.get(grandchild).unwrap().parent, Some(child));

        graph.destroy(parent);

        assert!(!graph.contains(parent));
        assert!(!graph.contains(child));
        assert!(!graph.contains(grandchild));
    }

    #[test]
    fn free_list_reuses_slots_with_bumped_generation() {
        let mut graph: Graph<TestNode> = Graph::new();

        let a = graph.insert(TestNode::new("a"), None);
        let b = graph.insert(TestNode::new("b"), None);
        let c = graph.insert(TestNode::new("c"), None);

        assert_eq!((a.slot, a.generation), (0, 0));
        assert_eq!((b.slot, b.generation), (1, 0));
        assert_eq!((c.slot, c.generation), (2, 0));

        graph.destroy(b);
        assert!(!graph.contains(b));

        // Next insert reuses slot 1 — but with a bumped generation, so the
        // old `b` NodeId no longer matches.
        let d = graph.insert(TestNode::new("d"), None);
        assert_eq!(d.slot, 1, "slot recycled");
        assert_eq!(d.generation, 1, "generation bumped on reuse");
        assert_eq!(graph.get(d).unwrap().payload.name, "d");

        // The old `b` NodeId still has generation 0; the slot is at
        // generation 1; lookup returns None. This is the ABA fix: stale
        // ids never silently resolve to their replacement.
        assert!(graph.get(b).is_none(), "stale NodeId does not alias new occupant");

        // Original slots a and c are untouched.
        assert_eq!(graph.get(a).unwrap().payload.name, "a");
        assert_eq!(graph.get(c).unwrap().payload.name, "c");
    }

    #[test]
    fn destroyed_subtree_detaches_from_parent() {
        // Hard-link cascade plus parent fix-up: after destroying a child, the
        // parent's `children` list no longer references it.
        let mut graph: Graph<TestNode> = Graph::new();
        let parent = graph.insert(TestNode::new("p"), None);
        let child_a = graph.insert(TestNode::new("a"), Some(parent));
        let child_b = graph.insert(TestNode::new("b"), Some(parent));

        assert_eq!(graph.get(parent).unwrap().children, vec![child_a, child_b]);

        graph.destroy(child_a);

        assert!(graph.contains(parent));
        assert!(!graph.contains(child_a));
        assert_eq!(graph.get(parent).unwrap().children, vec![child_b]);
    }

    #[test]
    fn stale_id_does_not_resolve_after_destroy() {
        // ABA hazard regression test. Holding a NodeId across a destroy +
        // re-insert at the same slot must not silently resolve to the new
        // occupant.
        let mut graph: Graph<TestNode> = Graph::new();
        let stale = graph.insert(TestNode::new("first"), None);

        graph.destroy(stale);
        assert!(graph.get(stale).is_none());

        let _replacement = graph.insert(TestNode::new("second"), None);
        // The replacement landed in the same slot; the stale id has the
        // old generation; `get` rejects it.
        assert!(graph.get(stale).is_none(), "stale id must not resolve to replacement");
    }
}

impl<P> Graph<P> {
    /// Preorder traversal of the hard-link subtrees rooted at `roots` —
    /// the sanctioned way to visit one file's nodes. Per ADR-0029 a
    /// file's roots own every declaration and use site in the file, so
    /// this is "iterate the file" without touching the rest of the
    /// arena (which [`Graph::iter`]'s docs forbid on hot paths).
    ///
    /// Stale ids are skipped via the generational [`Graph::get`]. The
    /// forest is acyclic by construction (parent set at insert, never
    /// mutated), so no cycle guard is needed.
    pub fn descendants<'a>(&'a self, roots: &[NodeId]) -> Descendants<'a, P> {
        Descendants {
            graph: self,
            stack: roots.iter().rev().copied().collect(),
        }
    }
}

/// Iterator over a hard-link subtree in preorder. See
/// [`Graph::descendants`].
pub struct Descendants<'a, P> {
    graph: &'a Graph<P>,
    stack: Vec<NodeId>,
}

impl<'a, P> Iterator for Descendants<'a, P> {
    type Item = (NodeId, &'a NodeData<P>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(id) = self.stack.pop() {
            if let Some(node) = self.graph.get(id) {
                self.stack.extend(node.children.iter().rev().copied());
                return Some((id, node));
            }
        }
        None
    }
}
