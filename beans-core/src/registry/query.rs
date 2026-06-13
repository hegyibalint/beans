//! Query types — what consumers interact with to look things up across
//! registries.
//!
//! Three layers, sized to caller need:
//!
//! 1. [`QueryResult`] — tri-state owned value: `None`, `One(NodeId)`,
//!    `Many(Vec<NodeId>)`. Cardinality at a glance; the zero/one cases
//!    never allocate; pattern matches at the call site make the
//!    cardinality explicit. Per ADR-0007 the `NodeId`s are generational
//!    handles, safe to hold across mutations.
//!
//! 2. [`Query<K>`] / [`Subscription<K>`] — the typestate split for a
//!    single-key lookup. `Query<K>` is stateless: just resolve. Calling
//!    `Query::subscribe(cb)` consumes the query and returns a
//!    `Subscription<K>` whose `Drop` unregisters automatically. The
//!    type *is* the lifecycle: a `Subscription` is, by construction,
//!    subscribed; a `Query` is, by construction, not.
//!
//! 3. [`FallbackSubscription<P, F>`] — the cross-language watch. Two
//!    typed `Subscription`s, primary-then-fallback resolve, cached
//!    invalidation, consumer subscribers via [`Watch`]. The "JVM
//!    fallback" pattern that recurs across every language module (per
//!    ADR-0008) named directly. No `Box<dyn _>`, no generics over
//!    arity — fixed two-Subscription shape, statically dispatched.
//!
//! Why no generic `MultiQuery<N>`: we considered it. In practice every
//! consulted-registries pattern in the project is fixed at construction
//! time, language by language. Naming the *one* recurring pattern
//! (`FallbackSubscription`) is more honest than abstracting over a
//! generality we don't have. When a second pattern materializes (e.g.,
//! completion's "merge across N JVM registries"), it gets its own named
//! type — designed when we know what its real shape needs to be, not
//! speculatively.

use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

use super::SimpleNamed;
use super::{Callback, Registry, SubscriptionId, RegistryKey};
use crate::graph::NodeId;

// =========================================================================
// QueryResult
// =========================================================================

/// Tri-state owned query result. Owns the matching `NodeId`s; no
/// borrows escape, so the caller can pattern-match, store, send across
/// channels, or `?`-chain through the graph for dereferencing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResult {
    None,
    One(NodeId),
    Many(Vec<NodeId>),
}

impl QueryResult {
    /// `Some(id)` if there is at least one match, `None` otherwise.
    /// Convenience for go-to-definition style callers that want a
    /// single representative.
    pub fn first(&self) -> Option<NodeId> {
        match self {
            QueryResult::None => None,
            QueryResult::One(id) => Some(*id),
            QueryResult::Many(ids) => ids.first().copied(),
        }
    }

    /// Consume into an owned `Vec<NodeId>`. Empty for `None`, length-1
    /// for `One`, the original Vec for `Many`. Convenience for callers
    /// that want a uniform iteration shape.
    pub fn all(self) -> Vec<NodeId> {
        match self {
            QueryResult::None => Vec::new(),
            QueryResult::One(id) => vec![id],
            QueryResult::Many(ids) => ids,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, QueryResult::None)
    }

    /// Number of matching providers.
    pub fn count(&self) -> usize {
        match self {
            QueryResult::None => 0,
            QueryResult::One(_) => 1,
            QueryResult::Many(ids) => ids.len(),
        }
    }
}

impl From<Vec<NodeId>> for QueryResult {
    fn from(v: Vec<NodeId>) -> Self {
        match v.len() {
            0 => QueryResult::None,
            1 => QueryResult::One(v[0]),
            _ => QueryResult::Many(v),
        }
    }
}

// =========================================================================
// Query<K> — stateless one-shot
// =========================================================================

/// A stateless lookup: a registry handle plus a key. Constructed via
/// [`Registry::query`]. Call [`resolve`](Self::resolve) for a one-shot
/// lookup, or [`subscribe`](Self::subscribe) to convert into an active
/// [`Subscription<K>`] that watches the key for changes.
///
/// The typestate split is deliberate: a `Query` is, by construction,
/// *not* subscribed; a `Subscription` is. There's no `Option<Id>` state
/// machine in the struct, no "is this watching or not" ambiguity at
/// every method call. The compiler enforces the lifecycle.
pub struct Query<K: RegistryKey> {
    registry: Registry<K>,
    key: K,
}

impl<K: RegistryKey> Query<K> {
    pub(crate) fn new(registry: Registry<K>, key: K) -> Self {
        Self { registry, key }
    }

    /// One-shot lookup. Returns the current providers without
    /// subscribing to changes.
    pub fn resolve(&self) -> QueryResult {
        QueryResult::from(self.registry.providers(&self.key))
    }

    /// Consume this query, register `cb` as a subscriber for its key,
    /// and return an active [`Subscription<K>`]. The subscription's
    /// `Drop` automatically removes the registration.
    pub fn subscribe(self, cb: Callback) -> Subscription<K> {
        let id = self.registry.subscribe_internal(self.key.clone(), cb);
        Subscription {
            registry: self.registry,
            key: self.key,
            id,
        }
    }
}

// =========================================================================
// Subscription<K> — active single-key watch with RAII cleanup
// =========================================================================

/// An active subscription on one registry's key. Constructed by
/// consuming a [`Query<K>`] via [`Query::subscribe`]. Holds a strong
/// clone of the registry handle so the registry can't be torn down
/// while a `Subscription` is alive; on `Drop` the registry's
/// subscriber list entry is removed.
///
/// Replaces the older `SubscriptionHandle<K>` — the typestate split
/// makes this struct itself the RAII anchor, no separate handle type
/// needed.
pub struct Subscription<K: RegistryKey> {
    registry: Registry<K>,
    key: K,
    id: SubscriptionId,
}

impl<K: RegistryKey> Subscription<K> {
    /// Look up providers for the watched key. Same shape as
    /// [`Query::resolve`]; available on `Subscription` because once
    /// you're watching, you'll usually want to read too.
    pub fn resolve(&self) -> QueryResult {
        QueryResult::from(self.registry.providers(&self.key))
    }
}

impl<K: RegistryKey> Drop for Subscription<K> {
    fn drop(&mut self) {
        // Strong Rc to the registry's inner — no Weak::upgrade dance,
        // the registry is alive by construction (we own a clone).
        self.registry.remove_subscription(&self.key, self.id);
    }
}

// `Subscription` is deliberately not [`Clone`]: each instance owns
// exactly one subscriber-list entry, and its `Drop` removes that one
// entry. Cloning would let two values believe they own the same
// subscription, and the second `Drop` would no-op on an already-empty
// slot — no soundness problem, but a confusing footgun.

// =========================================================================
// FallbackSubscription<P, F> — the cross-language two-key watch
// =========================================================================

/// Cache state for a [`FallbackSubscription`]. `Stale` means the next
/// `resolve` should walk the queries fresh; `Fresh` carries the last
/// computed value.
#[derive(Debug, Clone)]
enum Cached {
    Stale,
    Fresh(QueryResult),
}

/// Subscriber entry on a `FallbackSubscription`. Liveness flag
/// pattern: the [`Watch`] returned by `subscribe` shares an
/// `Rc<Cell<bool>>` with this entry; on `Watch` drop the flag flips
/// to `false` and the next fire prunes the entry.
struct WatchSubscriber {
    callback: Callback,
    alive: Rc<Cell<bool>>,
}

/// Consumer-side handle for [`FallbackSubscription::subscribe`]. Drop
/// stops further notifications; the entry on the parent
/// `FallbackSubscription` is pruned on its next fire.
#[derive(Debug)]
pub struct Watch {
    alive: Rc<Cell<bool>>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.alive.set(false);
    }
}

/// Cross-language two-key watch with primary-then-fallback resolve.
///
/// The recurring pattern across every JVM language: a use-site looks
/// for a definition first in its own language's registry, and falls
/// back to the JVM projection if missing. Two typed [`Subscription`]s,
/// fixed-arity, statically dispatched, no `Box<dyn _>`.
///
/// `P` is the primary (language-native) key type; `F` is the fallback
/// (typically a JVM projection key) type. Both subscriptions are wired
/// at construction; either's underlying provider-set change invalidates
/// the cache and fires consumer subscribers registered via
/// [`subscribe`](Self::subscribe).
pub struct FallbackSubscription<P, F>
where
    P: RegistryKey,
    F: RegistryKey,
{
    primary: Subscription<P>,
    fallback: Subscription<F>,
    cached: Rc<RefCell<Cached>>,
    consumer_subs: Rc<RefCell<Vec<WatchSubscriber>>>,
}

impl<P, F> FallbackSubscription<P, F>
where
    P: RegistryKey,
    F: RegistryKey,
{
    /// Construct a fallback subscription. Subscribes to both registries
    /// at construction; the cache is initially `Stale` so the first
    /// `resolve` walks fresh.
    pub fn new(
        primary_registry: &Registry<P>,
        primary_key: P,
        fallback_registry: &Registry<F>,
        fallback_key: F,
    ) -> Self {
        let cached = Rc::new(RefCell::new(Cached::Stale));
        let consumer_subs: Rc<RefCell<Vec<WatchSubscriber>>> =
            Rc::new(RefCell::new(Vec::new()));

        // The wrapping callback: invalidate own cache, then fire every
        // live consumer subscriber. Snapshot-and-release: clone the
        // live callback list under a brief borrow before invoking.
        let cached_for_cb = Rc::clone(&cached);
        let consumer_subs_for_cb = Rc::clone(&consumer_subs);
        let invalidation_cb: Callback = Rc::new(move || {
            *cached_for_cb.borrow_mut() = Cached::Stale;
            let to_fire: Vec<Callback> = {
                let mut list = consumer_subs_for_cb.borrow_mut();
                list.retain(|entry| entry.alive.get());
                list.iter().map(|e| Rc::clone(&e.callback)).collect()
            };
            for cb in to_fire {
                cb();
            }
        });

        let primary = primary_registry
            .query(primary_key)
            .subscribe(invalidation_cb.clone());
        let fallback = fallback_registry
            .query(fallback_key)
            .subscribe(invalidation_cb);

        Self {
            primary,
            fallback,
            cached,
            consumer_subs,
        }
    }

    /// Resolve with primary-then-fallback semantics. Returns the cached
    /// result if `Fresh`; otherwise walks the primary first, falls
    /// through to the fallback if the primary returned `None`, caches
    /// the result, and returns.
    pub fn resolve(&self) -> QueryResult {
        if let Cached::Fresh(ref r) = *self.cached.borrow() {
            return r.clone();
        }
        let result = match self.primary.resolve() {
            QueryResult::None => self.fallback.resolve(),
            hit => hit,
        };
        *self.cached.borrow_mut() = Cached::Fresh(result.clone());
        result
    }

    /// Subscribe to invalidation events on this fallback. The callback
    /// fires after the cache has been flipped to `Stale`, so a
    /// subscriber that re-reads via [`resolve`](Self::resolve) on its
    /// callback observes the new value.
    ///
    /// Returns a RAII [`Watch`]; drop it to stop receiving
    /// notifications.
    pub fn subscribe(&self, cb: Callback) -> Watch {
        let alive = Rc::new(Cell::new(true));
        self.consumer_subs.borrow_mut().push(WatchSubscriber {
            callback: cb,
            alive: Rc::clone(&alive),
        });
        Watch { alive }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Local stand-in keys. The engine is key-agnostic; the real
    // per-vertical keys live in the crates above (beans-lang-jvm,
    // beans-lang-java). The names mirror them so the test bodies read
    // like real call sites.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Fqn(String);
    impl Fqn {
        fn new(s: &str) -> Self {
            Self(s.to_string())
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum TypeRef {
        Void,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct JvmTypeKey(String);
    impl JvmTypeKey {
        fn new(s: &str) -> Self {
            Self(s.to_string())
        }
    }
    impl SimpleNamed for JvmTypeKey {
        fn simple_name(&self) -> &str {
            self.0.rsplit('.').next().unwrap_or(&self.0)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct JvmMethodKey(String, String, Vec<TypeRef>);
    impl JvmMethodKey {
        fn new(owner: &str, name: &str, params: Vec<TypeRef>) -> Self {
            Self(owner.to_string(), name.to_string(), params)
        }
    }
    impl SimpleNamed for JvmMethodKey {
        fn simple_name(&self) -> &str {
            &self.1
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct JavaSymbolKey(Fqn);
    impl JavaSymbolKey {
        fn new(f: Fqn) -> Self {
            Self(f)
        }
    }
    impl SimpleNamed for JavaSymbolKey {
        fn simple_name(&self) -> &str {
            self.0 .0.rsplit('.').next().unwrap_or(&self.0 .0)
        }
    }

    // ---- QueryResult ----

    #[test]
    fn query_result_from_vec() {
        let r: QueryResult = Vec::<NodeId>::new().into();
        assert!(matches!(r, QueryResult::None));

        let r: QueryResult = vec![NodeId::placeholder(1)].into();
        assert!(matches!(r, QueryResult::One(_)));

        let r: QueryResult = vec![NodeId::placeholder(1), NodeId::placeholder(2)].into();
        assert!(matches!(r, QueryResult::Many(_)));
    }

    #[test]
    fn query_result_count_and_first() {
        let r = QueryResult::None;
        assert_eq!(r.count(), 0);
        assert_eq!(r.first(), None);

        let r = QueryResult::One(NodeId::placeholder(7));
        assert_eq!(r.count(), 1);
        assert_eq!(r.first(), Some(NodeId::placeholder(7)));

        let r = QueryResult::Many(vec![NodeId::placeholder(7), NodeId::placeholder(8)]);
        assert_eq!(r.count(), 2);
        assert_eq!(r.first(), Some(NodeId::placeholder(7)));
    }

    // ---- Query<K> / Subscription<K> ----

    #[test]
    fn query_resolve_against_empty_and_filled_registry() {
        let registry: Registry<JvmTypeKey> = Registry::new();
        let key = JvmTypeKey::new("com.example.Foo");

        // Empty: query returns None.
        let q = registry.query(key.clone());
        assert!(matches!(q.resolve(), QueryResult::None));

        // After register: One.
        let _h = registry.register(key.clone(), NodeId::placeholder(1));
        let q = registry.query(key.clone());
        assert!(matches!(q.resolve(), QueryResult::One(_)));

        // After second: Many.
        let _h2 = registry.register(key.clone(), NodeId::placeholder(2));
        let q = registry.query(key);
        assert!(matches!(q.resolve(), QueryResult::Many(_)));
    }

    #[test]
    fn subscription_drop_unsubscribes_via_query() {
        // Pin the contract: a Subscription's Drop removes the
        // subscriber-list entry. Constructed via Query::subscribe, with
        // the public API surface; no direct registry.subscribe().
        let registry: Registry<JvmTypeKey> = Registry::new();
        let key = JvmTypeKey::new("com.example.Watched");

        let counter = Rc::new(Cell::new(0u32));
        let cb_counter = Rc::clone(&counter);
        let sub = registry.query(key.clone()).subscribe(Rc::new(move || {
            cb_counter.set(cb_counter.get() + 1);
        }));

        // Trigger a notification — subscriber fires.
        let _h = registry.register(key.clone(), NodeId::placeholder(1));
        assert_eq!(counter.get(), 1);

        // Drop the subscription — its Drop removes the registry entry.
        drop(sub);

        // Another mutation. Counter must not advance.
        let _h2 = registry.register(key.clone(), NodeId::placeholder(2));
        assert_eq!(counter.get(), 1, "dropped Subscription stops receiving notifications");
    }

    #[test]
    fn subscription_resolve_reflects_current_state() {
        // A Subscription can be queried for current providers without
        // creating a fresh Query; it's the same lookup on the same key.
        let registry: Registry<JvmTypeKey> = Registry::new();
        let key = JvmTypeKey::new("com.example.X");

        let sub = registry
            .query(key.clone())
            .subscribe(Rc::new(|| {}));

        assert!(matches!(sub.resolve(), QueryResult::None));

        let _h = registry.register(key, NodeId::placeholder(1));
        assert!(matches!(sub.resolve(), QueryResult::One(_)));
    }

    // ---- FallbackSubscription<P, F> ----

    #[test]
    fn fallback_resolves_primary_when_present() {
        let primary: Registry<JvmTypeKey> = Registry::new();
        let fallback: Registry<JvmMethodKey> = Registry::new();
        let primary_id = NodeId::placeholder(7);
        let _h = primary.register(JvmTypeKey::new("com.example.Service"), primary_id);

        let fb = FallbackSubscription::new(
            &primary,
            JvmTypeKey::new("com.example.Service"),
            &fallback,
            JvmMethodKey::new("com.example.Service", "process", vec![]),
        );

        match fb.resolve() {
            QueryResult::One(id) => assert_eq!(id, primary_id),
            other => panic!("expected One from primary, got {:?}", other),
        }
    }

    #[test]
    fn fallback_falls_through_when_primary_misses() {
        let primary: Registry<JvmTypeKey> = Registry::new();
        let fallback: Registry<JvmMethodKey> = Registry::new();
        let fallback_id = NodeId::placeholder(99);
        let _h = fallback.register(
            JvmMethodKey::new("com.example.Service", "process", vec![]),
            fallback_id,
        );

        let fb = FallbackSubscription::new(
            &primary,
            JvmTypeKey::new("com.example.Service"),
            &fallback,
            JvmMethodKey::new("com.example.Service", "process", vec![]),
        );

        match fb.resolve() {
            QueryResult::One(id) => assert_eq!(id, fallback_id),
            other => panic!("expected One from fallback, got {:?}", other),
        }
    }

    #[test]
    fn fallback_observes_primary_arrival_after_construction() {
        // Tier-2 contract: the cache invalidates when an underlying
        // provider set changes, without manual invalidate.
        let primary: Registry<JvmTypeKey> = Registry::new();
        let fallback: Registry<JvmMethodKey> = Registry::new();
        let fb = FallbackSubscription::new(
            &primary,
            JvmTypeKey::new("com.example.Service"),
            &fallback,
            JvmMethodKey::new("com.example.Service", "process", vec![]),
        );
        assert!(fb.resolve().is_empty());

        let new_id = NodeId::placeholder(42);
        let _h = primary.register(JvmTypeKey::new("com.example.Service"), new_id);

        // Without manual invalidate, the cache flipped to Stale via the
        // wrapping callback; next resolve recomputes and finds the new
        // primary provider.
        match fb.resolve() {
            QueryResult::One(id) => assert_eq!(id, new_id),
            other => panic!("expected One after registration, got {:?}", other),
        }
    }

    #[test]
    fn fallback_subscriber_fires_on_underlying_change() {
        // Consumer subscribes to the FallbackSubscription; underlying
        // registry mutates; consumer's callback fires.
        let primary: Registry<JvmTypeKey> = Registry::new();
        let fallback: Registry<JvmMethodKey> = Registry::new();
        let fb = FallbackSubscription::new(
            &primary,
            JvmTypeKey::new("com.example.Service"),
            &fallback,
            JvmMethodKey::new("com.example.Service", "process", vec![]),
        );

        let counter = Rc::new(Cell::new(0u32));
        let cb_counter = Rc::clone(&counter);
        let _watch = fb.subscribe(Rc::new(move || {
            cb_counter.set(cb_counter.get() + 1);
        }));

        let _h = primary.register(
            JvmTypeKey::new("com.example.Service"),
            NodeId::placeholder(1),
        );
        assert!(counter.get() > 0, "consumer's callback should fire");
    }

    #[test]
    fn fallback_watch_drop_stops_notifications() {
        let primary: Registry<JvmTypeKey> = Registry::new();
        let fallback: Registry<JvmMethodKey> = Registry::new();
        let fb = FallbackSubscription::new(
            &primary,
            JvmTypeKey::new("com.example.Service"),
            &fallback,
            JvmMethodKey::new("com.example.Service", "process", vec![]),
        );

        let counter = Rc::new(Cell::new(0u32));
        let cb_counter = Rc::clone(&counter);
        let watch = fb.subscribe(Rc::new(move || {
            cb_counter.set(cb_counter.get() + 1);
        }));

        let _h = primary.register(
            JvmTypeKey::new("com.example.Service"),
            NodeId::placeholder(1),
        );
        let after_first = counter.get();
        assert!(after_first > 0);

        drop(watch);

        // Another mutation. Counter must not advance — watch is dropped.
        let _h2 = primary.register(
            JvmTypeKey::new("com.example.Service"),
            NodeId::placeholder(2),
        );
        assert_eq!(
            counter.get(),
            after_first,
            "dropped Watch stops notifications"
        );
    }

    #[test]
    fn fallback_with_java_native_and_jvm_keys() {
        // The canonical cross-language pattern: Java-side native first,
        // JVM projection fallback.
        let java: Registry<JavaSymbolKey> = Registry::new();
        let jvm: Registry<JvmMethodKey> = Registry::new();

        let java_id = NodeId::placeholder(1);
        let jvm_id = NodeId::placeholder(2);
        let _hj = java.register(
            JavaSymbolKey::new(Fqn::new("com.example.Service.process")),
            java_id,
        );
        let _hv = jvm.register(
            JvmMethodKey::new("com.example.Service", "process", vec![TypeRef::Void]),
            jvm_id,
        );

        let fb: FallbackSubscription<JavaSymbolKey, JvmMethodKey> = FallbackSubscription::new(
            &java,
            JavaSymbolKey::new(Fqn::new("com.example.Service.process")),
            &jvm,
            JvmMethodKey::new("com.example.Service", "process", vec![TypeRef::Void]),
        );

        // Java-side wins because it's the primary.
        match fb.resolve() {
            QueryResult::One(id) => assert_eq!(id, java_id, "primary (Java) wins"),
            other => panic!("expected One, got {:?}", other),
        }
    }
}
