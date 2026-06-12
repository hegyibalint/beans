//! The registry layer — typed-key indices over graph nodes, plus the
//! query types that compose them.
//!
//! This is one of the engine's two pillars (the other is [`graph`]).
//! Graph owns nodes; registries index them. The two are orthogonal:
//! graph has zero knowledge of registries; registries depend on graph
//! only for `NodeId` and the `NodeHandle` marker trait.
//!
//! Layout:
//!
//! - This file ([`mod.rs`]) carries the [`Registries`] bag (one per
//!   [`crate::Beans`] instance, flat per-registry fields, not [`Clone`])
//!   and the [`Registry<K>`] primitive itself plus its provider RAII
//!   handle ([`ProviderHandle`], [`Callback`], [`SubscriptionId`]). Per
//!   ADR-0013 a registry stores *all* providers for a key; per ADR-0014
//!   RAII handles tie registration lifetime to node lifetime; per
//!   ADR-0015 the inner state is `Rc<RefCell<...>>` for re-entrant
//!   subscription support, with the snapshot-and-release pattern for
//!   callback safety. Per ADR-0008 `register` and the provider drop
//!   path auto-fire subscribers — no manual `notify` required for
//!   normal mutations.
//! - [`query`] holds the query types: [`QueryResult`] tri-state,
//!   [`Query<K>`] (stateless one-shot), [`Subscription<K>`] (active
//!   single-key watch — the RAII subscription, replacing the old
//!   `SubscriptionHandle<K>`), [`FallbackSubscription<P, F>`] (the
//!   cross-language two-key watch with primary-then-fallback resolve
//!   semantics), and [`Watch`] (the consumer-side handle returned by
//!   `FallbackSubscription::subscribe`).
//!
//! Subscriptions are constructed exclusively through
//! [`Registry::query(key).subscribe(cb)`] — the registry's underlying
//! `subscribe`/`remove_subscription` machinery is `pub(crate)`. This
//! keeps the public surface narrow: `Registry<K>::query` is the entry
//! point; everything else composes from there.
//!
//! Re-entrancy contract for subscription callbacks: callbacks may
//! freely *query* the registry that fired them (snapshot-and-release
//! handles that). Callbacks **must not** register or drop a provider
//! for the same key they are notifying on — that would re-enter
//! `notify` recursively for the same key, a programmer error the
//! registry does not detect. Cross-key mutation from inside a callback
//! is fine (and tested).
//!
//! Per ADR-0018: single-threaded. Nothing here is `Send`/`Sync`.
//!
//! [`graph`]: crate::graph

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::{Rc, Weak};

use crate::graph::{NodeHandle, NodeId};

pub mod query;

pub use query::{FallbackSubscription, Query, QueryResult, Subscription, Watch};

// Note: there is no bag-of-registries here. Each vertical crate owns
// its registry struct (`JvmRegistries` in beans-lang-jvm,
// `JavaRegistries` in beans-lang-java, ...) and the `beans` facade
// composes them. The engine provides only the `Registry<K>` primitive.

// =========================================================================
// Registry<K> — the typed multi-provider primitive
// =========================================================================

/// Identifier used internally by the registry to address a single
/// subscription. Allocated per-registry; not portable across registries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(u64);

/// Subscriber callback. `Rc<dyn Fn()>` so the registry can clone the
/// list out under a borrow and release the borrow before invoking — see
/// [`Registry::notify`].
pub type Callback = Rc<dyn Fn()>;

pub(crate) struct RegistryInner<K> {
    providers: HashMap<K, Vec<NodeId>>,
    subscribers: HashMap<K, Vec<(SubscriptionId, Callback)>>,
    next_id: u64,
}

impl<K> RegistryInner<K> {
    fn new() -> Self {
        Self {
            providers: HashMap::new(),
            subscribers: HashMap::new(),
            next_id: 0,
        }
    }

    fn alloc_subscription_id(&mut self) -> SubscriptionId {
        let id = SubscriptionId(self.next_id);
        self.next_id += 1;
        id
    }
}

impl<K: Eq + Hash> RegistryInner<K> {
    fn add_provider(&mut self, key: K, node: NodeId) {
        self.providers.entry(key).or_default().push(node);
    }

    fn remove_provider(&mut self, key: &K, node: NodeId) {
        // Remove only the *first* matching entry. Each ProviderHandle owns
        // exactly one registration; if a node registers twice for the same
        // key, two handles exist and dropping one must leave the other's
        // entry intact. `swap_remove` is fine because per ADR-0013 the
        // provider list has no significant order.
        if let Some(list) = self.providers.get_mut(key) {
            if let Some(pos) = list.iter().position(|n| *n == node) {
                list.swap_remove(pos);
            }
            if list.is_empty() {
                self.providers.remove(key);
            }
        }
    }

    fn add_subscription(&mut self, key: K, id: SubscriptionId, cb: Callback) {
        self.subscribers.entry(key).or_default().push((id, cb));
    }

    fn remove_subscription(&mut self, key: &K, id: SubscriptionId) {
        if let Some(list) = self.subscribers.get_mut(key) {
            list.retain(|(sub_id, _)| *sub_id != id);
            if list.is_empty() {
                self.subscribers.remove(key);
            }
        }
    }

    /// Snapshot of the current callback list for `key`. The caller drops
    /// the borrow over `self` before invoking the snapshot, so callbacks
    /// may freely re-enter the registry (snapshot-and-release per
    /// ADR-0015). Shared by [`Registry::notify`] and the provider RAII
    /// drop path so both go through the same re-entrancy-safe mechanism.
    fn snapshot_subscribers(&self, key: &K) -> Vec<Callback> {
        self.subscribers
            .get(key)
            .map(|v| v.iter().map(|(_, cb)| Rc::clone(cb)).collect())
            .unwrap_or_default()
    }
}

/// Multi-provider registry. Owns its inner state via `Rc<RefCell<_>>` so
/// the registry's storage can be shared cheaply between the registry
/// itself, the [`Query`]/[`Subscription`] objects it produces, and any
/// provider handles outstanding.
///
/// Cloning a `Registry` produces another strong reference to the same
/// underlying state — the same registry, two handles to it. This is how
/// nodes in the graph get a strong reference for registration and how
/// `Query`s carry a registry reference for later resolution.
pub struct Registry<K> {
    pub(crate) inner: Rc<RefCell<RegistryInner<K>>>,
}

impl<K> Default for Registry<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K> Clone for Registry<K> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<K> Registry<K> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(RegistryInner::new())),
        }
    }
}

impl<K: Eq + Hash> Registry<K> {
    /// Return all providers currently registered for `key`. Order is
    /// insertion order; per ADR-0013 this carries no semantic weight.
    ///
    /// Most consumers use [`Registry::query`] and the [`QueryResult`]
    /// tri-state instead of inspecting a `Vec`. `providers` is the
    /// raw-access form, used internally by [`Query::resolve`] /
    /// [`Subscription::resolve`].
    pub fn providers(&self, key: &K) -> Vec<NodeId> {
        self.inner
            .borrow()
            .providers
            .get(key)
            .cloned()
            .unwrap_or_default()
    }

    /// Fire all callbacks subscribed to `key`. Uses snapshot-and-release
    /// (ADR-0015): clone the callback list under a short borrow, drop
    /// the borrow, then invoke. Callbacks may freely re-enter the
    /// registry. Subscribers added during a callback are picked up on
    /// the *next* notification, not the current one.
    ///
    /// `register` and the [`ProviderHandle`] drop path call this
    /// automatically per ADR-0008, so consumers rarely need to invoke
    /// it manually. It remains public for non-mutation fan-outs.
    pub fn notify(&self, key: &K) {
        let callbacks = self.inner.borrow().snapshot_subscribers(key);
        for cb in callbacks {
            cb();
        }
    }
}

impl<K: Eq + Hash + Clone + 'static> Registry<K> {
    /// Construct a [`Query<K>`] for this key. The returned `Query` is
    /// stateless — it holds a (cheap) clone of this registry and the
    /// key. Call `resolve()` to look up providers, or `subscribe(cb)`
    /// to convert it into an active [`Subscription<K>`].
    pub fn query(&self, key: K) -> Query<K> {
        Query::new(self.clone(), key)
    }

    /// Register `node` as a provider for `key`. The returned handle's
    /// `Drop` removes the registration; store it on the node to bind
    /// registration lifetime to node lifetime.
    ///
    /// Per ADR-0008 every subscriber on `key` is notified after the
    /// provider is added, before this function returns. Callbacks run
    /// under the snapshot-and-release contract (see [`Self::notify`]).
    pub fn register(&self, key: K, node: NodeId) -> ProviderHandle<K> {
        self.inner.borrow_mut().add_provider(key.clone(), node);
        self.notify(&key);
        ProviderHandle {
            inner: Rc::downgrade(&self.inner),
            key,
            node,
        }
    }

    /// Subscribe `cb` to notifications on `key`, returning an opaque
    /// [`SubscriptionId`]. The caller is responsible for pairing this
    /// with a later [`Self::remove_subscription`] call when the
    /// subscription should end. In practice this is wired through
    /// [`Subscription::Drop`]; consumers of the registry construct
    /// subscriptions via [`Self::query`] then `subscribe`, never call
    /// this directly.
    pub(crate) fn subscribe_internal(&self, key: K, cb: Callback) -> SubscriptionId {
        let mut inner = self.inner.borrow_mut();
        let id = inner.alloc_subscription_id();
        inner.add_subscription(key, id, cb);
        id
    }

    /// Remove the subscription identified by `(key, id)`. Called by
    /// [`Subscription::Drop`].
    pub(crate) fn remove_subscription(&self, key: &K, id: SubscriptionId) {
        self.inner.borrow_mut().remove_subscription(key, id);
    }
}

// `NodeHandle` is defined in `crate::graph::arena` (next to its
// consumer `NodeData::handles`); the registry layer impls it for
// [`ProviderHandle`] so the engine can store provider registrations on
// nodes for cleanup. Subscriptions don't need this impl — they live
// inside [`Query`]/[`Subscription`]/[`FallbackSubscription`] objects,
// not on `NodeData::handles`.
impl<K: Eq + Hash> NodeHandle for ProviderHandle<K> {}

/// RAII registration. Drop unregisters this `(key, node)` from the registry.
/// If the registry has already been dropped, the upgrade fails and Drop
/// is a no-op — gracefully handling tear-down ordering (ADR-0015).
///
/// Deliberately not [`Clone`]: each handle owns exactly one provider
/// entry and dropping it removes one entry from the registry's provider
/// list. Cloning would let two handles believe they own the same
/// registration, and dropping both would over-remove. Per ADR-0014 the
/// handle is the *one* RAII anchor for its registration.
#[derive(Debug)]
pub struct ProviderHandle<K: Eq + Hash> {
    inner: Weak<RefCell<RegistryInner<K>>>,
    key: K,
    node: NodeId,
}

impl<K: Eq + Hash> Drop for ProviderHandle<K> {
    fn drop(&mut self) {
        let Some(inner) = self.inner.upgrade() else {
            // Registry already torn down — nothing to remove and nobody
            // to notify. Per ADR-0015 this is a safe no-op rather than a
            // panic.
            return;
        };
        inner.borrow_mut().remove_provider(&self.key, self.node);
        // Per ADR-0008, fire subscribers after the mutation. Use the
        // shared snapshot-and-release helper so callbacks may re-enter
        // the registry safely (ADR-0015).
        let callbacks = inner.borrow().snapshot_subscribers(&self.key);
        for cb in callbacks {
            cb();
        }
    }
}

#[cfg(test)]
mod tests {
    //! Two flavours of registry test:
    //!
    //! * **Pure registry**: provider/subscription lifecycle, RAII drop
    //!   semantics, snapshot-and-release re-entrancy, auto-notify on
    //!   register/drop. No `Graph` needed; `NodeId::placeholder(_)`
    //!   stand-ins are used for slots since the registry treats `NodeId`s
    //!   as opaque values it stores and compares.
    //! * **Graph-integrated**: a `HandleNode` payload holds a real
    //!   `ProviderHandle` / `Subscription`; destroying the node drops
    //!   the handles, and the test asserts on registry state.

    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;
    use crate::graph::Graph;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestKey(&'static str);

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct OtherKey(&'static str);

    /// Minimal payload used by graph-integrated registry tests.
    struct HandleNode {
        provider: Option<ProviderHandle<TestKey>>,
        subscription: Option<Subscription<TestKey>>,
        notifications: Rc<Cell<u32>>,
    }

    impl HandleNode {
        fn new() -> Self {
            Self {
                provider: None,
                subscription: None,
                notifications: Rc::new(Cell::new(0)),
            }
        }
    }

    #[test]
    fn provider_handle_drop_unregisters() {
        // Graph-integrated: a node holds a ProviderHandle; destroying the
        // node drops the handle, which removes the registry entry.
        let mut graph: Graph<HandleNode> = Graph::new();
        let registry: Registry<TestKey> = Registry::new();

        let id = graph.insert(HandleNode::new(), None);
        let key = TestKey("foo");
        {
            let node = graph.get_mut(id).unwrap();
            node.payload.provider = Some(registry.register(key.clone(), id));
        }

        assert_eq!(registry.providers(&key), vec![id]);

        graph.destroy(id);

        assert!(registry.providers(&key).is_empty());
    }

    #[test]
    fn subscription_drop_unsubscribes() {
        // Graph-integrated mirror: a node holds a Subscription. Destroying
        // the node drops it; later notifies do not reach the (now-dropped)
        // callback.
        let mut graph: Graph<HandleNode> = Graph::new();
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("watch-me");

        let id = graph.insert(HandleNode::new(), None);
        let counter = graph.get(id).unwrap().payload.notifications.clone();
        {
            let cb_counter = counter.clone();
            let cb = Rc::new(move || {
                cb_counter.set(cb_counter.get() + 1);
            });
            let node = graph.get_mut(id).unwrap();
            node.payload.subscription = Some(registry.query(key.clone()).subscribe(cb));
        }

        // Sanity: callback fires while subscribed.
        registry.notify(&key);
        assert_eq!(counter.get(), 1);

        graph.destroy(id);

        // Counter is dropped along with the node, but the registry should no
        // longer carry the callback. Notifying again must not panic.
        registry.notify(&key);
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn registering_and_dropping_a_provider_fires_subscribers() {
        // Per ADR-0008 the registry auto-fires subscribers on every change to
        // the provider set: register fires after add, the ProviderHandle drop
        // path fires after remove. Manual notify still works for non-mutation
        // fan-outs.
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("fanout");

        let counter = Rc::new(Cell::new(0u32));
        let cb_counter = counter.clone();
        let _sub = registry.query(key.clone()).subscribe(Rc::new(move || {
            cb_counter.set(cb_counter.get() + 1);
        }));

        let provider = registry.register(key.clone(), NodeId::placeholder(42));
        assert_eq!(counter.get(), 1, "subscriber fires on provider registration");

        drop(provider);
        assert_eq!(counter.get(), 2, "subscriber fires on provider drop");

        // Manual notify still fans out to existing subscribers.
        registry.notify(&key);
        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn snapshot_and_release_allows_reentrant_query() {
        // A subscriber callback that reads the providers map on the same
        // registry must not RefCell-panic. The snapshot-and-release pattern
        // copies the callback list out under a short borrow and drops the
        // borrow before invoking, so the callback is free to re-enter.
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("reentrant");

        let _provider_a = registry.register(key.clone(), NodeId::placeholder(1));
        let _provider_b = registry.register(key.clone(), NodeId::placeholder(2));

        let observed: Rc<Cell<usize>> = Rc::new(Cell::new(0));
        let observed_in_cb = observed.clone();
        let registry_in_cb = registry.clone();
        let key_in_cb = key.clone();
        let _sub = registry.query(key.clone()).subscribe(Rc::new(move || {
            // Re-enter: query the same registry from inside the callback.
            let providers = registry_in_cb.providers(&key_in_cb);
            observed_in_cb.set(providers.len());
        }));

        registry.notify(&key);

        assert_eq!(observed.get(), 2);
    }

    #[test]
    fn provider_handle_outliving_registry_does_not_panic() {
        // Per ADR-0015 the provider handle holds a Weak; if the registry
        // is dropped first the handle's Drop no-ops via failed
        // `Weak::upgrade`. (Subscriptions hold a strong Rc instead, so
        // they keep the registry alive — that case is intentionally
        // different and is exercised by `subscription_drop_unsubscribes`.)
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("drop-order");

        let provider = registry.register(key.clone(), NodeId::placeholder(7));

        drop(registry);
        // If the Weak upgrade did not gracefully no-op we would either
        // panic or borrow-mut a dangling cell.
        drop(provider);
    }

    #[test]
    fn duplicate_provider_registration_drops_one_at_a_time() {
        // RAII invariant: each ProviderHandle owns exactly one entry. If a node
        // registers twice for the same key, dropping one handle must leave the
        // other entry intact.
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("dup");
        let node = NodeId::placeholder(7);

        let h1 = registry.register(key.clone(), node);
        let h2 = registry.register(key.clone(), node);

        assert_eq!(registry.providers(&key), vec![node, node]);

        drop(h1);
        assert_eq!(registry.providers(&key), vec![node]);

        drop(h2);
        assert!(registry.providers(&key).is_empty());
    }

    #[test]
    fn notify_callback_can_register_in_a_different_registry() {
        // The snapshot-and-release pattern (ADR-0015) handles re-entrancy on
        // the *same* registry — a callback querying the registry that fired
        // it. The cross-registry case is also supported: a callback that
        // mutates a *different* registry while the first is mid-notify must
        // not RefCell-panic and must leave both registries internally
        // consistent.
        let primary: Registry<TestKey> = Registry::new();
        let secondary: Registry<OtherKey> = Registry::new();
        let key_a = TestKey("source");
        let key_b = OtherKey("derived");

        // Subscriber on `primary` registers a provider in `secondary` when
        // notified. Holding the handle in a Cell so the closure can move it
        // and the test can still observe `secondary`'s state afterwards.
        let derived_handle: Rc<RefCell<Option<ProviderHandle<OtherKey>>>> =
            Rc::new(RefCell::new(None));
        let secondary_in_cb = secondary.clone();
        let key_b_in_cb = key_b.clone();
        let dh_in_cb = Rc::clone(&derived_handle);
        let _sub = primary.query(key_a.clone()).subscribe(Rc::new(move || {
            // Re-enter the *secondary* registry from inside the primary's
            // notification path.
            let h = secondary_in_cb.register(key_b_in_cb.clone(), NodeId::placeholder(123));
            *dh_in_cb.borrow_mut() = Some(h);
        }));

        // Before notify: secondary is empty.
        assert!(secondary.providers(&key_b).is_empty());

        primary.notify(&key_a);

        // After notify: secondary has the provider the callback registered,
        // and the primary's internals are still usable (re-entrancy didn't
        // wedge it).
        assert_eq!(
            secondary.providers(&key_b),
            vec![NodeId::placeholder(123)]
        );
        let _ = primary.register(key_a.clone(), NodeId::placeholder(7));

        // Drop the derived handle — secondary cleans up.
        *derived_handle.borrow_mut() = None;
        assert!(secondary.providers(&key_b).is_empty());
    }
}
