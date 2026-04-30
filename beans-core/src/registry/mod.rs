//! Registry: typed-key index over graph nodes, with subscription / notification.
//!
//! The registry layer is **orthogonal to the graph**. The graph owns nodes
//! (slots, hard links, lifecycle); a registry stores `NodeId`s the graph
//! has minted, indexed by a typed key the consumer chooses. A consumer
//! could in principle build its own indexing scheme over `NodeId`s; this
//! module is the canonical one because it implements ADR-0008's
//! subscription contract.
//!
//! Per ADR-0013: registries store *all* providers for a key; precedence is
//! a resolution-layer concern. Per ADR-0014: provider registrations and
//! subscriptions are returned as RAII handles whose `Drop` cleans up.
//! Per ADR-0015: registries are `Rc<RefCell<RegistryInner<K>>>`; handles
//! hold a `Weak` so the registry can be torn down while handles still
//! exist. Notify follows the snapshot-and-release pattern to support
//! re-entrant callbacks safely under `RefCell`.
//!
//! Per ADR-0008 the registry auto-fires subscribers on every provider-set
//! mutation: [`Registry::register`] notifies after adding,
//! [`ProviderHandle`]'s drop path notifies after removing. Subscribers
//! receive a wake on every change to the provider set for their key; the
//! callback decides whether anything needs to be done (typically by
//! invalidating a cached resolution and letting the next pull recompute).
//! Manual [`Registry::notify`] remains available for consumers that need
//! to fan out a non-mutation signal.
//!
//! Re-entrancy contract: subscriber callbacks may freely query the
//! registry (snapshot-and-release handles that). Callbacks **must not**
//! register or drop a provider for the same key they are notifying on,
//! since that would re-enter notify recursively for the same key — a
//! programmer error the registry does not detect. Cross-key mutation
//! from inside a callback is fine (and tested).
//!
//! Per ADR-0018: single-threaded. Nothing here is `Send`/`Sync`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::{Rc, Weak};

use crate::graph::{NodeHandle, NodeId};

#[cfg(test)]
mod tests;

/// Identifier used internally by the registry to address a single
/// subscription. Allocated per-registry; not portable across registries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(u64);

/// Subscriber callback. `Rc<dyn Fn()>` so the registry can clone the list
/// out under a borrow and release the borrow before invoking — see
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
/// handles can carry a `Weak` back-reference and the registry can be torn
/// down independently of outstanding handles.
///
/// Cloning a `Registry` produces another strong reference to the same
/// underlying state — the same registry, two handles to it. This is how
/// nodes in the graph get a strong reference for registration while their
/// stored handles only carry a `Weak`.
pub struct Registry<K> {
    inner: Rc<RefCell<RegistryInner<K>>>,
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
    pub fn query(&self, key: &K) -> Vec<NodeId> {
        self.inner
            .borrow()
            .providers
            .get(key)
            .cloned()
            .unwrap_or_default()
    }

    /// Fire all callbacks subscribed to `key`. Uses snapshot-and-release
    /// (ADR-0015): clone the callback list under a short borrow, drop the
    /// borrow, then invoke. Callbacks may freely re-enter the registry.
    /// Subscribers added during a callback are picked up on the *next*
    /// notification, not the current one.
    ///
    /// `register` and the [`ProviderHandle`] drop path call this
    /// automatically per ADR-0008, so consumers rarely need to invoke it
    /// manually. It remains public for non-mutation fan-outs.
    pub fn notify(&self, key: &K) {
        let callbacks = self.inner.borrow().snapshot_subscribers(key);
        for cb in callbacks {
            cb();
        }
    }
}

impl<K: Eq + Hash + Clone> Registry<K> {
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

    /// Subscribe `cb` to notifications on `key`. The returned handle's
    /// `Drop` unsubscribes.
    pub fn subscribe(&self, key: K, cb: Callback) -> SubscriptionHandle<K> {
        let id = {
            let mut inner = self.inner.borrow_mut();
            let id = inner.alloc_subscription_id();
            inner.add_subscription(key.clone(), id, cb);
            id
        };
        SubscriptionHandle {
            inner: Rc::downgrade(&self.inner),
            key,
            id,
        }
    }
}

// `NodeHandle` itself is defined in `crate::graph::arena` (next to its
// consumer `NodeData::handles`); the registry layer just impls it for the
// two RAII handle types it produces. This keeps `graph` free of any
// dependency on `registry`.
impl<K: Eq + Hash> NodeHandle for ProviderHandle<K> {}
impl<K: Eq + Hash> NodeHandle for SubscriptionHandle<K> {}

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

/// RAII subscription. Drop removes this subscription from the registry.
/// Same single-owner contract as [`ProviderHandle`]; not [`Clone`].
#[derive(Debug)]
pub struct SubscriptionHandle<K: Eq + Hash> {
    inner: Weak<RefCell<RegistryInner<K>>>,
    key: K,
    id: SubscriptionId,
}

impl<K: Eq + Hash> Drop for SubscriptionHandle<K> {
    fn drop(&mut self) {
        if let Some(reg) = self.inner.upgrade() {
            reg.borrow_mut().remove_subscription(&self.key, self.id);
        }
    }
}
