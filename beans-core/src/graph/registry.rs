//! Registry, ProviderHandle, SubscriptionHandle.
//!
//! Per ADR-0013: registries store *all* providers for a key; precedence is
//! a resolution-layer concern. Per ADR-0014: provider registrations and
//! subscriptions are returned as RAII handles whose `Drop` cleans up.
//! Per ADR-0015: registries are `Rc<RefCell<RegistryInner<K>>>`; handles
//! hold a `Weak` so the registry can be torn down while handles still
//! exist. Notify follows the snapshot-and-release pattern to support
//! re-entrant callbacks safely under `RefCell`.
//!
//! Per ADR-0018: single-threaded. Nothing here is `Send`/`Sync`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::{Rc, Weak};

use crate::graph::arena::NodeId;

/// Identifier used internally by the registry to address a single
/// subscription. Allocated per-registry; not portable across registries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(u64);

/// Subscriber callback. `Rc<dyn Fn()>` so the registry can clone the list
/// out under a borrow and release the borrow before invoking — see
/// `Registry::notify`.
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
        if let Some(list) = self.providers.get_mut(key) {
            list.retain(|n| *n != node);
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

impl<K: Eq + Hash + Clone> Registry<K> {
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

    /// Register `node` as a provider for `key`. The returned handle's
    /// `Drop` removes the registration; store it on the node to bind
    /// registration lifetime to node lifetime.
    pub fn register(&self, key: K, node: NodeId) -> ProviderHandle<K> {
        self.inner.borrow_mut().add_provider(key.clone(), node);
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

    /// Fire all callbacks subscribed to `key`. Uses snapshot-and-release
    /// (ADR-0015): clone the callback list under a short borrow, drop the
    /// borrow, then invoke. Callbacks may freely re-enter the registry.
    /// Subscribers added during a callback are picked up on the *next*
    /// notification, not the current one.
    pub fn notify(&self, key: &K) {
        let callbacks: Vec<Callback> = {
            let inner = self.inner.borrow();
            inner
                .subscribers
                .get(key)
                .map(|v| v.iter().map(|(_, cb)| Rc::clone(cb)).collect())
                .unwrap_or_default()
        };
        for cb in callbacks {
            cb();
        }
    }
}

/// RAII registration. Drop unregisters this `(key, node)` from the registry.
/// If the registry has already been dropped, the upgrade fails and Drop
/// is a no-op — gracefully handling tear-down ordering (ADR-0015).
pub struct ProviderHandle<K: Eq + Hash> {
    inner: Weak<RefCell<RegistryInner<K>>>,
    key: K,
    node: NodeId,
}

impl<K: Eq + Hash> Drop for ProviderHandle<K> {
    fn drop(&mut self) {
        if let Some(reg) = self.inner.upgrade() {
            reg.borrow_mut().remove_provider(&self.key, self.node);
        }
    }
}

/// RAII subscription. Drop removes this subscription from the registry.
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
