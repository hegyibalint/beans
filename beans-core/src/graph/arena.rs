//! Graph arena and `NodeData<P>`.
//!
//! Per ADR-0007: `NodeId` is a runtime arena index, not a stable external
//! identity. It is preserved verbatim across snapshot save/load (the entire
//! arena round-trips); it is *not* meaningful across rebuilds.
//!
//! Per ADR-0006: hard links are stored as `Vec<NodeId>` on the parent. When
//! a node is destroyed, the GC walks its hard-link subtree post-order and
//! frees every descendant.

use crate::graph::cache_state::{CacheState, Generation};

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

/// Runtime arena index into a `Graph<P>`. Per ADR-0007 this is an internal
/// identifier — external APIs speak in registry keys, not `NodeId`. The
/// inner `u64` is `pub(crate)` so snapshot save/load and intra-crate tests
/// can construct ids freely; consumers outside `beans-core` use `raw()` to
/// observe the bits but cannot fabricate them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub(crate) u64);

impl NodeId {
    /// Observe the underlying `u64`. Useful for logging and snapshot
    /// serialization; not a stable identity across rebuilds.
    pub fn raw(self) -> u64 {
        self.0
    }

    #[allow(dead_code)] // symmetry with raw(); used by future snapshot loader.
    pub(crate) fn from_raw(raw: u64) -> Self {
        NodeId(raw)
    }

    fn slot(self) -> usize {
        self.0 as usize
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
    pub state: CacheState,
    pub payload: P,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    /// RAII handles installed by [`NodeBehavior::on_created`](crate::graph::NodeBehavior::on_created)
    /// after the node is in the arena. Stored as `Box<dyn NodeHandle>` because
    /// the engine has no per-key knowledge; each impl drops itself.
    pub handles: Vec<Box<dyn NodeHandle>>,
}

// `Vec<Box<dyn NodeHandle>>` doesn't impl `Debug`, so we manually derive a
// `Debug` that elides the handles. They're opaque to anything except
// their own Drop impls.
impl<P: std::fmt::Debug> std::fmt::Debug for NodeData<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeData")
            .field("state", &self.state)
            .field("payload", &self.payload)
            .field("parent", &self.parent)
            .field("children", &self.children)
            .field("handles", &format_args!("[{} handles]", self.handles.len()))
            .finish()
    }
}

/// Single-payload graph arena. Owns a flat `Vec<Option<NodeData<P>>>`.
/// Free slots are tracked in a `Vec<usize>` and reused on the next insert.
///
/// The engine is generic over `P` so the same machinery serves the JVM
/// payload union, test payloads, and any future tagged variant.
pub struct Graph<P> {
    slots: Vec<Option<NodeData<P>>>,
    free: Vec<usize>,
    current_gen: Generation,
}

impl<P: std::fmt::Debug> std::fmt::Debug for Graph<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Graph")
            .field("slots", &self.slots)
            .field("free", &self.free)
            .field("current_gen", &self.current_gen)
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
            current_gen: Generation::ZERO,
        }
    }

    /// Allocate a new slot and store `payload` in it. If `parent` is set,
    /// the new node is appended to the parent's `children` (a hard link).
    /// Initial state is `Stale` — the value has been *placed* but not yet
    /// validated by a `mark_fresh` call.
    pub fn insert(&mut self, payload: P, parent: Option<NodeId>) -> NodeId {
        let data = NodeData {
            state: CacheState::Stale,
            payload,
            parent,
            children: Vec::new(),
            handles: Vec::new(),
        };

        let slot_index = match self.free.pop() {
            Some(idx) => {
                debug_assert!(self.slots[idx].is_none());
                self.slots[idx] = Some(data);
                idx
            }
            None => {
                let idx = self.slots.len();
                self.slots.push(Some(data));
                idx
            }
        };

        let id = NodeId(slot_index as u64);

        if let Some(parent_id) = parent {
            // Parent must exist; treating a missing parent as a programmer error.
            // Index via `get_mut` so an out-of-range parent NodeId surfaces
            // through the same descriptive expect rather than the slice's
            // bounds-check message.
            self.slots
                .get_mut(parent_id.slot())
                .and_then(|s| s.as_mut())
                .expect("insert: parent NodeId references an empty slot")
                .children
                .push(id);
        }

        id
    }

    pub fn get(&self, id: NodeId) -> Option<&NodeData<P>> {
        self.slots.get(id.slot()).and_then(|s| s.as_ref())
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut NodeData<P>> {
        self.slots.get_mut(id.slot()).and_then(|s| s.as_mut())
    }

    /// True if the slot currently holds a node. False after `destroy`.
    pub fn contains(&self, id: NodeId) -> bool {
        self.get(id).is_some()
    }

    /// Recursive, post-order destroy: every descendant is freed before the
    /// node itself. Per ADR-0014 each [`NodeData`]'s [`handles`](NodeData::handles)
    /// vec drops as the slot is freed, and each handle's `Drop` removes
    /// its registry entry.
    ///
    /// If `id` has a parent, it is also detached from the parent's
    /// `children` list. Calling `destroy` on a non-existent slot is a no-op.
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
        // each contained `Box<dyn NodeHandle>` runs its `Drop` impl —
        // ProviderHandle/SubscriptionHandle remove their registry
        // entries via the snapshot-and-release pattern (ADR-0015).
        let slot = id.slot();
        self.slots[slot] = None;
        self.free.push(slot);
    }

    /// Bump the global generation counter and mark the given node `Stale`.
    /// Bumps the counter even if the node was already stale — staleness
    /// is a per-node fact but the generation is the engine-wide clock.
    pub fn mark_stale(&mut self, id: NodeId) {
        self.current_gen = self.current_gen.bump();
        if let Some(node) = self.get_mut(id) {
            node.state = CacheState::Stale;
        }
    }

    /// Record a freshly-computed value at the given generation.
    pub fn mark_fresh(&mut self, id: NodeId, generation: Generation) {
        if let Some(node) = self.get_mut(id) {
            node.state = CacheState::Fresh(generation);
        }
    }

    pub fn current_generation(&self) -> Generation {
        self.current_gen
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
            .filter_map(|(idx, slot)| slot.as_ref().map(|n| (NodeId(idx as u64), n)))
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
        assert_eq!(id, NodeId(0));
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
    fn free_list_reuses_slots() {
        let mut graph: Graph<TestNode> = Graph::new();

        let a = graph.insert(TestNode::new("a"), None);
        let b = graph.insert(TestNode::new("b"), None);
        let c = graph.insert(TestNode::new("c"), None);

        assert_eq!(a, NodeId(0));
        assert_eq!(b, NodeId(1));
        assert_eq!(c, NodeId(2));

        graph.destroy(b);
        assert!(!graph.contains(b));

        // The next insert should land in slot 1, the freed one.
        let d = graph.insert(TestNode::new("d"), None);
        assert_eq!(d, NodeId(1));
        assert_eq!(graph.get(d).unwrap().payload.name, "d");

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
    fn generation_is_monotonic_across_mark_stale() {
        // `mark_stale` lives on the arena (it owns `current_gen`), so its
        // monotonicity test belongs here. The `Generation` value-type
        // tests live in cache_state.
        let mut graph: Graph<TestNode> = Graph::new();
        let id = graph.insert(TestNode::new("gen"), None);

        let g0 = graph.current_generation();

        graph.mark_stale(id);
        let g1 = graph.current_generation();
        assert!(g1 > g0);
        assert_eq!(graph.get(id).unwrap().state, CacheState::Stale);

        graph.mark_fresh(id, g1);
        assert_eq!(graph.get(id).unwrap().state, CacheState::Fresh(g1));
        // mark_fresh does not touch the global counter.
        assert_eq!(graph.current_generation(), g1);

        graph.mark_stale(id);
        let g2 = graph.current_generation();
        assert!(g2 > g1);

        graph.mark_stale(id);
        let g3 = graph.current_generation();
        assert!(g3 > g2);
    }

    #[test]
    fn fresh_graph_starts_at_generation_zero() {
        let graph: Graph<TestNode> = Graph::new();
        assert_eq!(graph.current_generation(), Generation::ZERO);
    }
}
