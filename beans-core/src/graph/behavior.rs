//! `NodeBehavior` trait for payload-driven lifecycle hooks.
//!
//! Most cleanup happens via `Drop` on RAII handles stored in the payload
//! itself (ADR-0014). The trait is here for the cases that don't fit RAII:
//! a node may want to *create* its registrations after it has a stable
//! `NodeId` (because the `NodeId` is what the registry stores), and that
//! requires a hook that runs *after* the arena slot is filled.
//!
//! Call shape (intended use, established by the test suite):
//!
//! 1. Caller constructs the payload.
//! 2. `Graph::insert(payload, parent)` slots it and returns the `NodeId`.
//! 3. Caller invokes `payload.on_created(id, &mut ctx)` via a `&mut`
//!    borrow into the arena (`graph.get_mut(id).unwrap()`).
//!    Inside, the payload calls `registry.register(...)` /
//!    `registry.subscribe(...)` and stores the returned RAII handles
//!    on `self`.
//! 4. On destroy, `payload.on_destroyed(id, &mut ctx)` runs *before* the
//!    slot is cleared (so it can still read the payload). Most cleanup
//!    needs nothing here — the RAII handles drop with the payload.
//!
//! `Ctx` is the consumer's registry struct (e.g. `Registries`). The trait
//! is generic so the engine doesn't need to know about specific
//! registries.
//!
//! For this milestone the trait is *unused* by `Graph` itself — the test
//! payload calls its own lifecycle methods directly. Once we have multiple
//! polymorphic node types in one arena (Java + Kotlin + JVM in beans-core
//! proper), `Graph::insert` can be wrapped with a helper that invokes
//! `NodeBehavior::on_created` automatically.

use crate::graph::arena::NodeId;

pub trait NodeBehavior {
    /// Registry/registries struct the consumer threads through lifecycle
    /// calls. Typically `Registries` from a higher layer; for tests it is
    /// whatever bag-of-registries the test wants.
    type Ctx;

    /// Called after the node is in the arena and has a valid `NodeId`.
    /// Implementations register provider/subscription handles via the
    /// registries in `ctx` and store the returned handles on `self`.
    fn on_created(&mut self, id: NodeId, ctx: &mut Self::Ctx);

    /// Called before the node's slot is freed. Most consumers leave this
    /// as the default no-op and rely on RAII handle drop.
    fn on_destroyed(&mut self, _id: NodeId, _ctx: &mut Self::Ctx) {}
}
