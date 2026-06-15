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
//! - This file ([`mod.rs`]) carries the [`Registry<K>`] primitive
//!   itself plus its provider RAII handle ([`ProviderHandle`],
//!   [`Callback`], [`SubscriptionId`]). There is no bag-of-registries
//!   here — each vertical owns its registry struct and the `beans`
//!   facade composes them (see the note below). Per
//!   ADR-0013 a registry stores *all* providers for a key; per ADR-0014
//!   RAII handles tie registration lifetime to node lifetime; per
//!   ADR-0015 the inner state is `Rc<RefCell<...>>` for re-entrant
//!   subscription support, with the snapshot-and-release pattern for
//!   callback safety. Per ADR-0008 `register` and the provider drop
//!   path auto-fire subscribers — no manual `notify` required for
//!   normal mutations. [`Registry::begin_batch`]/[`Registry::commit_batch`]
//!   coalesce those notifications: inside a batch, mutations and queries
//!   stay live but subscriber callbacks defer to the outermost commit and
//!   fire once per changed key (used by bulk indexing).
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
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::rc::{Rc, Weak};

use crate::graph::{NodeHandle, NodeId};

pub mod query;

pub use query::{FallbackSubscription, Query, QueryResult, Subscription, Watch};

// Note: there is no bag-of-registries here. Each vertical crate owns
// its registry struct (`JvmRegistries` in beans-lang-jvm,
// `JavaRegistries` in beans-lang-java, ...) and the `beans` facade
// composes them. The engine provides only the `Registry<K>` primitive.

/// The last meaningful segment of a key's qualified name — the
/// *source* simple name (`com.example.Service` → `Service`).
///
/// Part of the universal key contract: JVM naming is hierarchical
/// everywhere, so every key can answer this, and the registry
/// maintains an eager simple-name index on the back of it (measured
/// in: the scan version cost 8ms per query at gradle/master scale,
/// paid per unresolved name per diagnostics pass). Implementations
/// must return source simple names, not binary-name segments (a
/// future Scala `Config$` projection answers `Config`) — consumers
/// build user-facing edits (imports) from these.
pub trait SimpleNamed {
    fn simple_name(&self) -> &str;
}

/// The universal registry key contract: hashable, cloneable, owning,
/// and simple-named (ADR-0012 typed keys + the eager simple-name
/// index). Blanket-implemented — defining a key type means satisfying
/// these bounds, nothing more.
pub trait RegistryKey: Eq + Hash + Clone + SimpleNamed + 'static {}

impl<T: Eq + Hash + Clone + SimpleNamed + 'static> RegistryKey for T {}

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
    /// Eager reverse index: simple name → provider nodes. Maintained
    /// by add/remove_provider; queried by
    /// [`Registry::query_simple_name`] in O(1). Stores `NodeId`s, not
    /// keys: consumers resolve through the graph anyway (kind filter,
    /// FQN from the payload header), and cloned keys would duplicate
    /// every FQN string a second time per registry.
    by_simple_name: HashMap<String, Vec<NodeId>>,
    subscribers: HashMap<K, Vec<(SubscriptionId, Callback)>>,
    next_id: u64,

    /// Notification-batch nesting depth. Zero means callbacks fire
    /// immediately (the default); positive means we are inside one or
    /// more `begin_batch`/`commit_batch` spans and subscriber callbacks
    /// are deferred. See [`Registry::begin_batch`].
    batch_depth: usize,
    /// Keys whose subscriber notification should fire at the end of the
    /// current batch. Drained and fired once each at the outermost
    /// `commit_batch`. A `HashSet` because a key touched N times in a
    /// batch still fires once.
    pending_notifications: HashSet<K>,
}

impl<K> RegistryInner<K> {
    fn new() -> Self {
        Self {
            providers: HashMap::new(),
            by_simple_name: HashMap::new(),
            subscribers: HashMap::new(),
            next_id: 0,
            batch_depth: 0,
            pending_notifications: HashSet::new(),
        }
    }

    fn alloc_subscription_id(&mut self) -> SubscriptionId {
        let id = SubscriptionId(self.next_id);
        self.next_id += 1;
        id
    }
}

impl<K: RegistryKey> RegistryInner<K> {
    fn add_provider(&mut self, key: K, node: NodeId) {
        // get_mut-then-insert instead of entry(): the entry API would
        // allocate the name String on every call, including the common
        // bucket-exists case.
        if let Some(bucket) = self.by_simple_name.get_mut(key.simple_name()) {
            bucket.push(node);
        } else {
            self.by_simple_name
                .insert(key.simple_name().to_string(), vec![node]);
        }
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
        if let Some(bucket) = self.by_simple_name.get_mut(key.simple_name()) {
            if let Some(pos) = bucket.iter().position(|n| *n == node) {
                bucket.swap_remove(pos);
            }
            if bucket.is_empty() {
                self.by_simple_name.remove(key.simple_name());
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

impl<K: RegistryKey> Registry<K> {
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

    /// Provider nodes of every key whose
    /// [`SimpleNamed::simple_name`] equals `name` — one hash lookup
    /// against the eager index `add_provider`/`remove_provider`
    /// maintain. Returns `NodeId`s; callers filter and read FQNs
    /// through the graph (which they need for kind checks anyway).
    ///
    /// History: this began as a key-set scan (measured-first posture).
    /// At gradle/master scale the scan cost 8ms per call and the
    /// missing-import rule pays one call per unresolved name per
    /// recompute — 233ms/file. The index was the measured upgrade;
    /// storing ids instead of cloned keys was the memory follow-up.
    pub fn query_simple_name(&self, name: &str) -> Vec<NodeId> {
        self.inner
            .borrow()
            .by_simple_name
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    /// Fire all callbacks subscribed to `key` — *or*, inside a batch,
    /// record `key` to be fired once at the outermost `commit_batch`.
    ///
    /// Outside a batch this fans out immediately, using
    /// snapshot-and-release (ADR-0015): clone the callback list under a
    /// short borrow, drop the borrow, then invoke. Callbacks may freely
    /// re-enter the registry; subscribers added during a callback are
    /// picked up on the *next* notification, not the current one.
    ///
    /// Inside a batch (`batch_depth > 0`) this is batch-aware: it does
    /// not fan out, it enqueues `key` into `pending_notifications` with
    /// the same once-per-key coalescing as mutation-triggered
    /// notifications. This upholds the batch contract that no subscriber
    /// observes intermediate integration state. See [`Self::begin_batch`].
    ///
    /// `register` and the [`ProviderHandle`] drop path route through the
    /// same fire-or-defer helper per ADR-0008, so consumers rarely need
    /// to invoke this manually. It remains the public non-mutation
    /// fan-out API.
    pub fn notify(&self, key: &K) {
        notify_or_defer(&self.inner, key);
    }

    /// Open a notification batch. While any batch is open, provider
    /// mutations still apply immediately and one-shot queries
    /// ([`Self::providers`], [`Self::query_simple_name`]) see current
    /// state — only subscriber callbacks are deferred. Each key whose
    /// provider set changes, or that receives an explicit [`Self::notify`],
    /// is recorded once; [`Self::commit_batch`] fires those deferred
    /// notifications.
    ///
    /// Batches nest: `begin_batch` increments a depth counter and only
    /// the outermost `commit_batch` drains and fires. This is
    /// notification coalescing, not a transaction — there is no rollback
    /// and no isolation guarantee, just delayed observer dispatch.
    pub fn begin_batch(&self) {
        self.inner.borrow_mut().batch_depth += 1;
    }

    /// Close a notification batch opened by [`Self::begin_batch`]. The
    /// outermost commit (depth returning to zero) fires each recorded
    /// key's subscribers exactly once; inner commits only decrement the
    /// depth.
    ///
    /// The flush is a single observer boundary: every pending key's
    /// callback list is snapshotted *before* any callback runs, then the
    /// borrow is released and the snapshots fire (snapshot-and-release,
    /// ADR-0015). So a callback that mutates the subscriber list of
    /// another key still in this flush affects only *future*
    /// notifications, never the rest of this flush — the outcome doesn't
    /// depend on the (unordered) key iteration.
    ///
    /// Calling `commit_batch` without a matching [`Self::begin_batch`] is
    /// a programmer error and panics.
    pub fn commit_batch(&self) {
        // Snapshot every pending key's callbacks under one borrow, then
        // release it before firing. Capturing all snapshots up front is
        // what makes the flush a single observer boundary; releasing
        // before firing lets callbacks re-enter (and re-entrant
        // mutations, now at depth 0, fan out immediately).
        let notifications: Vec<Vec<Callback>> = {
            let mut inner = self.inner.borrow_mut();
            assert!(
                inner.batch_depth > 0,
                "commit_batch without a matching begin_batch"
            );
            inner.batch_depth -= 1;
            if inner.batch_depth > 0 {
                return;
            }
            let pending = std::mem::take(&mut inner.pending_notifications);
            pending
                .iter()
                .map(|key| inner.snapshot_subscribers(key))
                .collect()
        };
        for callbacks in notifications {
            for cb in callbacks {
                cb();
            }
        }
    }
}

/// Fire-or-defer the subscriber notification for `key`. Outside a batch
/// it snapshots the callbacks (releasing the borrow first, per ADR-0015)
/// and fans out immediately; inside a batch it records `key` for the
/// outermost [`Registry::commit_batch`] to fire. Shared by
/// [`Registry::register`], [`Registry::notify`], and the
/// [`ProviderHandle`] drop path so all three observe batch mode
/// identically and never fire while holding the `RefCell` borrow.
fn notify_or_defer<K: RegistryKey>(inner: &Rc<RefCell<RegistryInner<K>>>, key: &K) {
    let callbacks = {
        let mut guard = inner.borrow_mut();
        if guard.batch_depth > 0 {
            guard.pending_notifications.insert(key.clone());
            return;
        }
        guard.snapshot_subscribers(key)
    };
    for cb in callbacks {
        cb();
    }
}

impl<K: RegistryKey> Registry<K> {
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
        notify_or_defer(&self.inner, &key);
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
impl<K: RegistryKey> NodeHandle for ProviderHandle<K> {}

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
pub struct ProviderHandle<K: RegistryKey> {
    inner: Weak<RefCell<RegistryInner<K>>>,
    key: K,
    node: NodeId,
}

impl<K: RegistryKey> Drop for ProviderHandle<K> {
    fn drop(&mut self) {
        let Some(inner) = self.inner.upgrade() else {
            // Registry already torn down — nothing to remove and nobody
            // to notify. Per ADR-0015 this is a safe no-op rather than a
            // panic.
            return;
        };
        inner.borrow_mut().remove_provider(&self.key, self.node);
        // Per ADR-0008, fire subscribers after the mutation — through the
        // shared fire-or-defer helper so a drop inside a batch coalesces
        // like any other mutation, and so callbacks fired now run under
        // snapshot-and-release and may re-enter safely (ADR-0015).
        notify_or_defer(&inner, &self.key);
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

    impl SimpleNamed for TestKey {
        fn simple_name(&self) -> &str {
            self.0
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct OtherKey(&'static str);

    impl SimpleNamed for OtherKey {
        fn simple_name(&self) -> &str {
            self.0
        }
    }

    #[test]
    fn simple_name_index_tracks_provider_lifecycle() {
        let registry: Registry<TestKey> = Registry::new();
        // TestKey's simple name is its whole string; two keys, same name
        // is impossible here, so use two distinct names and assert
        // bucket isolation plus RAII removal.
        let a = TestKey("Service");
        let b = TestKey("Repository");

        let ha = registry.register(a, NodeId::placeholder(1));
        let _hb = registry.register(b, NodeId::placeholder(2));

        assert_eq!(
            registry.query_simple_name("Service"),
            vec![NodeId::placeholder(1)]
        );
        assert_eq!(
            registry.query_simple_name("Repository"),
            vec![NodeId::placeholder(2)]
        );
        assert!(registry.query_simple_name("Missing").is_empty());

        // Dropping the provider handle must erase the index entry too —
        // a stale index would offer imports for deleted types.
        drop(ha);
        assert!(registry.query_simple_name("Service").is_empty());
    }

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
        assert_eq!(
            counter.get(),
            1,
            "subscriber fires on provider registration"
        );

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
        assert_eq!(secondary.providers(&key_b), vec![NodeId::placeholder(123)]);
        let _ = primary.register(key_a.clone(), NodeId::placeholder(7));

        // Drop the derived handle — secondary cleans up.
        *derived_handle.borrow_mut() = None;
        assert!(secondary.providers(&key_b).is_empty());
    }

    /// Subscribe a counter to `key` and return (subscription, counter).
    /// Hold the subscription for the test's lifetime.
    fn counting_sub(
        registry: &Registry<TestKey>,
        key: &TestKey,
    ) -> (Subscription<TestKey>, Rc<Cell<u32>>) {
        let counter = Rc::new(Cell::new(0u32));
        let cb_counter = counter.clone();
        let sub = registry
            .query(key.clone())
            .subscribe(Rc::new(move || cb_counter.set(cb_counter.get() + 1)));
        (sub, counter)
    }

    #[test]
    fn register_during_batch_defers_until_commit() {
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("Batched");
        let (_sub, counter) = counting_sub(&registry, &key);

        registry.begin_batch();
        let _h = registry.register(key.clone(), NodeId::placeholder(1));
        assert_eq!(
            counter.get(),
            0,
            "register inside a batch must not fire subscribers"
        );

        registry.commit_batch();
        assert_eq!(
            counter.get(),
            1,
            "the deferred notification fires at commit"
        );
    }

    #[test]
    fn provider_drop_during_batch_defers_until_commit() {
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("DropBatched");
        // Register before subscribing so the registration's own notify
        // doesn't perturb the count we assert on.
        let handle = registry.register(key.clone(), NodeId::placeholder(1));
        let (_sub, counter) = counting_sub(&registry, &key);

        registry.begin_batch();
        drop(handle);
        assert_eq!(
            counter.get(),
            0,
            "a provider drop inside a batch defers its notification"
        );

        registry.commit_batch();
        assert_eq!(
            counter.get(),
            1,
            "the deferred drop notification fires at commit"
        );
    }

    #[test]
    fn repeated_mutations_to_a_key_fire_once_per_batch() {
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("Coalesced");
        let (_sub, counter) = counting_sub(&registry, &key);

        registry.begin_batch();
        let h1 = registry.register(key.clone(), NodeId::placeholder(1));
        let h2 = registry.register(key.clone(), NodeId::placeholder(2));
        drop(h1);
        registry.notify(&key);
        assert_eq!(counter.get(), 0, "nothing fires mid-batch");

        registry.commit_batch();
        assert_eq!(
            counter.get(),
            1,
            "four touches to one key coalesce into a single notification"
        );
        // Hold h2 until after the assertion so its drop (which fires
        // immediately, now outside the batch) doesn't skew the count.
        drop(h2);
    }

    #[test]
    fn queries_see_provider_changes_before_commit() {
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("Visible");

        registry.begin_batch();
        let _h = registry.register(key.clone(), NodeId::placeholder(7));
        // Only notifications defer; the provider state itself is live.
        assert_eq!(registry.providers(&key), vec![NodeId::placeholder(7)]);
        assert_eq!(
            registry.query_simple_name("Visible"),
            vec![NodeId::placeholder(7)]
        );
        registry.commit_batch();
    }

    #[test]
    fn nested_batches_fire_only_on_outermost_commit() {
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("Nested");
        let (_sub, counter) = counting_sub(&registry, &key);

        registry.begin_batch();
        registry.begin_batch();
        let _h = registry.register(key.clone(), NodeId::placeholder(1));

        registry.commit_batch();
        assert_eq!(
            counter.get(),
            0,
            "an inner commit must not fire — still inside the outer batch"
        );

        registry.commit_batch();
        assert_eq!(
            counter.get(),
            1,
            "the outermost commit fires the deferred notification once"
        );
    }

    #[test]
    #[should_panic(expected = "commit_batch without a matching begin_batch")]
    fn commit_without_begin_panics() {
        let registry: Registry<TestKey> = Registry::new();
        registry.commit_batch();
    }

    #[test]
    fn manual_notify_is_batch_aware() {
        let registry: Registry<TestKey> = Registry::new();
        let key = TestKey("ManualNotify");
        let (_sub, counter) = counting_sub(&registry, &key);

        // Outside a batch, notify fans out immediately.
        registry.notify(&key);
        assert_eq!(counter.get(), 1);

        // Inside a batch, a manual notify coalesces to commit — the batch
        // contract (no subscriber sees intermediate state) has no bypass.
        registry.begin_batch();
        registry.notify(&key);
        registry.notify(&key);
        assert_eq!(
            counter.get(),
            1,
            "manual notify inside a batch does not fan out immediately"
        );

        registry.commit_batch();
        assert_eq!(
            counter.get(),
            2,
            "the coalesced manual notify fires once at commit"
        );
    }

    #[test]
    fn commit_flush_is_one_observer_boundary() {
        // A callback fired during the flush may subscribe to another key
        // that is *also* pending in the same batch. Because commit
        // snapshots every pending key's callbacks before firing any, that
        // late subscriber must not fire within this flush — regardless of
        // the (unordered) key iteration. Without the up-front snapshot the
        // outcome would be `HashSet`-order-dependent.
        let registry: Registry<TestKey> = Registry::new();
        let key_a = TestKey("A");
        let key_b = TestKey("B");

        // B's pre-existing subscriber, plus a slot to hold a subscription
        // added from inside A's callback (kept alive past the callback).
        let (_sub_b, b_count) = counting_sub(&registry, &key_b);
        let late_count = Rc::new(Cell::new(0u32));
        let late_slot: Rc<RefCell<Option<Subscription<TestKey>>>> = Rc::new(RefCell::new(None));

        let registry_in_cb = registry.clone();
        let key_b_in_cb = key_b.clone();
        let late_count_in_cb = late_count.clone();
        let late_slot_in_cb = Rc::clone(&late_slot);
        let _sub_a = registry.query(key_a.clone()).subscribe(Rc::new(move || {
            let lc = late_count_in_cb.clone();
            let sub = registry_in_cb
                .query(key_b_in_cb.clone())
                .subscribe(Rc::new(move || lc.set(lc.get() + 1)));
            *late_slot_in_cb.borrow_mut() = Some(sub);
        }));

        registry.begin_batch();
        let _ha = registry.register(key_a.clone(), NodeId::placeholder(1));
        let _hb = registry.register(key_b.clone(), NodeId::placeholder(2));
        registry.commit_batch();

        assert_eq!(b_count.get(), 1, "B's pre-existing subscriber fires once");
        assert_eq!(
            late_count.get(),
            0,
            "a subscriber added during the flush does not fire within the same flush"
        );

        // It is an ordinary subscriber from here on.
        registry.notify(&key_b);
        assert_eq!(
            late_count.get(),
            1,
            "the late subscriber fires on the next notification"
        );
    }
}
