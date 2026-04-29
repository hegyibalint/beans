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
//! 5. On destroy, `payload.on_destroyed(id, &ctx)` runs *before* the
//!    slot is cleared. Most consumers leave this as the default no-op
//!    and rely on the handles' RAII drop.
//!
//! Returning handles by value rather than mutating the engine through
//! a context method keeps the trait simple — there is no
//! "get-storage-then-push" surface to maintain — and matches the ADR
//! intent that handles travel through the payload's hands once before
//! settling on `NodeData`.
//!
//! `Ctx` is the consumer's registry struct (e.g.
//! [`Registries`](crate::Registries)). The engine never names specific
//! registry keys.
//!
//! [`NodeData`]: crate::graph::NodeData
//! [`NodeData::handles`]: crate::graph::NodeData::handles

use crate::graph::arena::NodeId;
use crate::graph::registry::NodeHandle;

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

    /// Called before the node's slot is freed. Most consumers leave
    /// this as the default no-op and rely on the handles' RAII drop.
    fn on_destroyed(&self, _id: NodeId, _ctx: &Self::Ctx) {}
}
