//! `NodeBehavior` trait for payload-driven lifecycle hooks.
//!
//! Per ADR-0014 RAII registration handles live on [`NodeData`], not on
//! the payload itself. The behavior trait gives the payload a chance to
//! produce those handles after the arena slot is filled (because the
//! `NodeId` is what each registry stores). The engine collects the
//! returned handles into [`NodeData::handles`] and drops them when the
//! slot is freed.
//!
//! Call shape (intended use):
//!
//! 1. Caller constructs the payload.
//! 2. [`Graph::insert(payload, parent)`](crate::graph::Graph::insert)
//!    slots it and returns the [`NodeId`].
//! 3. Caller invokes `payload.on_created(id, &ctx)` to obtain the
//!    `Vec<Box<dyn NodeHandle>>` of registration handles.
//! 4. Caller stores the returned vector into
//!    `graph.get_mut(id).unwrap().handles`.
//!
//! Returning handles by value rather than mutating the engine through
//! a context method keeps the trait simple — there is no
//! "get-storage-then-push" surface to maintain — and matches the ADR
//! intent that handles travel through the payload's hands once before
//! settling on `NodeData`.
//!
//! Cleanup is RAII per ADR-0014: dropping `NodeData` drops `handles`,
//! and each handle's `Drop` impl removes its registry entry. There is
//! no `on_destroyed` hook; if a payload needs custom teardown, encode it
//! in a `NodeHandle` impl whose `Drop` does the work.
//!
//! `Ctx` is the consumer's registry struct (e.g.
//! [`Registries`](crate::Registries)). The engine never names specific
//! registry keys.
//!
//! [`NodeData`]: crate::graph::NodeData
//! [`NodeData::handles`]: crate::graph::NodeData::handles

use crate::graph::arena::{NodeHandle, NodeId};

pub trait NodeBehavior {
    /// Registry/registries struct the consumer threads through
    /// lifecycle calls. Typically [`Registries`](crate::Registries)
    /// from a higher layer; for tests it is whatever bag-of-registries
    /// the test wants.
    type Ctx;

    /// Called after the node is in the arena and has a valid
    /// [`NodeId`]. Implementations register provider/subscription
    /// handles via the registries in `ctx` and return them; the engine
    /// stores the vector on the node and drops it on destroy.
    ///
    /// Returning an empty `Vec` is the right default for payloads that
    /// don't register anywhere — parameter nodes, leaf bookkeeping, and
    /// so on.
    fn on_created(&self, id: NodeId, ctx: &Self::Ctx) -> Vec<Box<dyn NodeHandle>>;
}
