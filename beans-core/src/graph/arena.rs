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
/// Provider/subscription handles are *stored inside the payload itself*
/// (per ADR-0014), so dropping the payload runs the RAII cleanup. We do
/// not carry handle vectors at this level because we do not know the
/// registry types at the engine layer.
#[derive(Debug)]
pub struct NodeData<P> {
    pub state: CacheState,
    pub payload: P,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

/// Single-payload graph arena. Owns a flat `Vec<Option<NodeData<P>>>`.
/// Free slots are tracked in a `Vec<usize>` and reused on the next insert.
///
/// The engine is generic over `P` so the same machinery serves the JVM
/// payload union, test payloads, and any future tagged variant.
#[derive(Debug)]
pub struct Graph<P> {
    slots: Vec<Option<NodeData<P>>>,
    free: Vec<usize>,
    current_gen: Generation,
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
            self.slots[parent_id.slot()]
                .as_mut()
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
    /// node itself. Per ADR-0014 the payload's RAII handles run their
    /// `Drop` impls during this walk and unregister themselves from any
    /// registry they joined.
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

        // Now drop the node itself; its payload's Drop runs here, which is
        // where RAII handles unregister.
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
}
