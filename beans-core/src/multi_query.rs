//! `MultiQuery` — subscription-backed, cached multi-registry lookup.
//!
//! Per ADR-0008 a use-site that wants to resolve "this name in any of
//! these registries, in priority order" stores the *question*, not the
//! answer. `MultiQuery` is that question made into a stored value:
//!
//! - It owns a `Vec<RegistryQuery>` — each variant is a typed (registry,
//!   key) pair. Closed enum, exhaustive: adding a new registry adds one
//!   variant and the compiler flags every match.
//! - It subscribes to each underlying registry on construction. When
//!   any underlying provider set changes, the cache flips to `Stale`
//!   and the consumer's subscribers fire.
//! - It exposes the same `subscribe(cb) -> SubscriptionHandle` API as
//!   `Registry<K>`, so consumers compose uniformly: subscribe to the
//!   MultiQuery as if it were any other registry-shaped thing.
//!
//! Reading the value: [`MultiQuery::query`] returns a `QueryResult`,
//! priority-ordered first-match across the underlying registries.
//! Cache is recomputed on first read after `Stale`; subsequent reads
//! return the cached value until the next mutation flips it.
//!
//! Per ADR-0007 the `NodeId`s in `QueryResult` are generational handles
//! — safe to hold across mutations and to dereference later through
//! the graph's generation-validated `get` (returns `None` if the slot
//! was destroyed before the consumer dereferences).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::beans::Beans;
use crate::jvm::{JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey};
use crate::query::QueryResult;
use crate::registry::Callback;

#[cfg(feature = "java")]
use crate::languages::java::JavaSymbolKey;

/// One typed (registry, key) lookup. Closed enum: one variant per
/// registry the project knows about. New registries add a variant; the
/// compiler flags every match.
#[derive(Debug, Clone)]
pub enum RegistryQuery {
    JvmType(JvmTypeKey),
    JvmMethod(JvmMethodKey),
    JvmField(JvmFieldKey),
    JvmConstructor(JvmConstructorKey),
    JvmPackage(PackageKey),
    #[cfg(feature = "java")]
    JavaSymbol(JavaSymbolKey),
}

impl RegistryQuery {
    /// Resolve this query against `beans`'s registries. Returns the
    /// providers (raw form). Used by [`MultiQuery`] for cache
    /// recomputation; tests can use it directly to avoid having to
    /// construct a full `MultiQuery` for one-shot lookups.
    pub fn providers(&self, beans: &Beans) -> Vec<crate::graph::NodeId> {
        match self {
            RegistryQuery::JvmType(k) => beans.registries.jvm_types.providers(k),
            RegistryQuery::JvmMethod(k) => beans.registries.jvm_methods.providers(k),
            RegistryQuery::JvmField(k) => beans.registries.jvm_fields.providers(k),
            RegistryQuery::JvmConstructor(k) => beans.registries.jvm_constructors.providers(k),
            RegistryQuery::JvmPackage(k) => beans.registries.jvm_packages.providers(k),
            #[cfg(feature = "java")]
            RegistryQuery::JavaSymbol(k) => beans.registries.java_symbols.providers(k),
        }
    }

    /// Subscribe `cb` to the underlying registry's notifications for
    /// this query's key. The returned handle's `Drop` unsubscribes.
    /// Used by [`MultiQuery::new`] to wire push-based invalidation.
    pub fn subscribe(&self, beans: &Beans, cb: Callback) -> Box<dyn crate::graph::NodeHandle> {
        match self {
            RegistryQuery::JvmType(k) => Box::new(beans.registries.jvm_types.subscribe(k.clone(), cb)),
            RegistryQuery::JvmMethod(k) => Box::new(beans.registries.jvm_methods.subscribe(k.clone(), cb)),
            RegistryQuery::JvmField(k) => Box::new(beans.registries.jvm_fields.subscribe(k.clone(), cb)),
            RegistryQuery::JvmConstructor(k) => Box::new(beans.registries.jvm_constructors.subscribe(k.clone(), cb)),
            RegistryQuery::JvmPackage(k) => Box::new(beans.registries.jvm_packages.subscribe(k.clone(), cb)),
            #[cfg(feature = "java")]
            RegistryQuery::JavaSymbol(k) => Box::new(beans.registries.java_symbols.subscribe(k.clone(), cb)),
        }
    }
}

/// Cache state for `MultiQuery`. `Stale` means "next read should walk
/// the queries fresh"; `Fresh` carries the last computed value.
#[derive(Debug, Clone)]
enum Cached {
    Stale,
    Fresh(QueryResult),
}

/// Subscriber entry on a `MultiQuery`. Liveness flag pattern: the
/// `SubscriptionHandle` returned by `MultiQuery::subscribe` holds an
/// `Rc<Cell<bool>>` shared with this entry; on handle drop the flag
/// flips to `false` and the next fire prunes the entry. Same pattern
/// the underlying `Registry<K>` uses internally for subscriptions.
struct MultiSubscriber {
    callback: Callback,
    alive: Rc<Cell<bool>>,
}

/// RAII subscription handle for `MultiQuery::subscribe`. Drop flips
/// the liveness flag to `false`; the MultiQuery prunes lazily on the
/// next fire for that subscriber's slot.
#[derive(Debug)]
pub struct MultiSubscriptionHandle {
    alive: Rc<Cell<bool>>,
}

impl Drop for MultiSubscriptionHandle {
    fn drop(&mut self) {
        self.alive.set(false);
    }
}

/// Subscription-backed, cached multi-registry lookup.
///
/// Construction takes `&mut Beans` because subscribing to each
/// underlying registry mutates the registry's subscriber list. After
/// construction, `query` takes `&Beans` (read-only); the cache is
/// invalidated through the subscription callbacks, not through the
/// query call.
pub struct MultiQuery {
    queries: Vec<RegistryQuery>,
    cached: Rc<RefCell<Cached>>,
    /// Subscriptions on each underlying registry. RAII: dropped when the
    /// MultiQuery drops, removing the registry-side entries. Stored as
    /// `Box<dyn NodeHandle>` because each variant of `RegistryQuery`
    /// produces a different `SubscriptionHandle<K>` type.
    _internal_subs: Vec<Box<dyn crate::graph::NodeHandle>>,
    /// Subscribers to *this* MultiQuery — consumers that registered via
    /// [`subscribe`](Self::subscribe). Wrapping callbacks for the
    /// internal subscriptions hold an `Rc` clone of this and fire each
    /// live subscriber on every underlying mutation.
    subscribers: Rc<RefCell<Vec<MultiSubscriber>>>,
}

impl MultiQuery {
    /// Construct a `MultiQuery` against the given list of priority-
    /// ordered queries. Subscribes to each underlying registry so the
    /// cache invalidates on any provider-set change.
    pub fn new(beans: &Beans, queries: Vec<RegistryQuery>) -> Self {
        let cached = Rc::new(RefCell::new(Cached::Stale));
        let subscribers: Rc<RefCell<Vec<MultiSubscriber>>> =
            Rc::new(RefCell::new(Vec::new()));

        // Wire each underlying subscription. The callback captures the
        // cache + subscriber list and (a) flips cache to Stale, then
        // (b) fires every live subscriber. Snapshot-and-release: clone
        // out the live callback list under a brief borrow before
        // invoking, so subscribers can re-enter (the RefCell on
        // subscribers itself is the only re-entrancy concern).
        let subs: Vec<Box<dyn crate::graph::NodeHandle>> = queries
            .iter()
            .map(|q| {
                let cached_for_cb = Rc::clone(&cached);
                let subscribers_for_cb = Rc::clone(&subscribers);
                let cb: Callback = Rc::new(move || {
                    *cached_for_cb.borrow_mut() = Cached::Stale;
                    let to_fire: Vec<Callback> = {
                        let mut list = subscribers_for_cb.borrow_mut();
                        list.retain(|entry| entry.alive.get());
                        list.iter().map(|e| Rc::clone(&e.callback)).collect()
                    };
                    for cb in to_fire {
                        cb();
                    }
                });
                q.subscribe(beans, cb)
            })
            .collect();

        Self {
            queries,
            cached,
            _internal_subs: subs,
            subscribers,
        }
    }

    /// Resolve this query against `beans`. Returns the cached result if
    /// `Fresh`, otherwise walks the queries in priority order, takes
    /// the first non-empty answer, caches it, and returns.
    ///
    /// First-match semantics: per ADR-0008's resolution mode, returns
    /// the first registry's answer; subsequent registries are ignored.
    /// Use [`providers_all`](Self::providers_all) for the merge-all
    /// (completion) shape.
    pub fn query(&self, beans: &Beans) -> QueryResult {
        // Read cache under a short borrow; clone out if Fresh.
        if let Cached::Fresh(ref r) = *self.cached.borrow() {
            return r.clone();
        }
        // Compute. Don't hold a borrow over `cached` during recompute —
        // a subscription callback firing mid-walk could try to mutate
        // `cached` (for nested MultiQueries, etc) and we want the
        // RefCell free.
        let mut result = QueryResult::None;
        for q in &self.queries {
            let providers = q.providers(beans);
            if !providers.is_empty() {
                result = QueryResult::from(providers);
                break;
            }
        }
        *self.cached.borrow_mut() = Cached::Fresh(result.clone());
        result
    }

    /// Resolve every underlying query and concatenate results in query
    /// order. Per ADR-0008 the merge does not de-duplicate. Not cached
    /// — completion answers change too often for a stale Vec to be
    /// useful.
    pub fn providers_all(&self, beans: &Beans) -> Vec<crate::graph::NodeId> {
        self.queries.iter().flat_map(|q| q.providers(beans)).collect()
    }

    /// Subscribe to this MultiQuery's invalidations. The callback fires
    /// after the cache has been flipped to `Stale`, so a subscriber
    /// that re-reads via [`query`](Self::query) on its callback sees
    /// the new value.
    ///
    /// Returns a RAII handle; drop it to stop receiving notifications.
    /// Same shape as [`Registry::subscribe`](crate::registry::Registry::subscribe).
    pub fn subscribe(&self, cb: Callback) -> MultiSubscriptionHandle {
        let alive = Rc::new(Cell::new(true));
        self.subscribers.borrow_mut().push(MultiSubscriber {
            callback: cb,
            alive: Rc::clone(&alive),
        });
        MultiSubscriptionHandle { alive }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::NodeId;

    #[test]
    fn multi_query_returns_first_match() {
        let beans = Beans::new();
        let id = NodeId::placeholder(7);
        let _h = beans
            .registries
            .jvm_types
            .register(JvmTypeKey::new("com.example.Service"), id);

        let mq = MultiQuery::new(
            &beans,
            vec![
                RegistryQuery::JvmType(JvmTypeKey::new("com.example.Service")),
            ],
        );

        match mq.query(&beans) {
            QueryResult::One(found) => assert_eq!(found, id),
            other => panic!("expected One, got {:?}", other),
        }
    }

    #[test]
    fn multi_query_falls_through_priority_order() {
        let beans = Beans::new();
        let kotlin_id = NodeId::placeholder(1);
        // Only the JVM-types registry has a provider; the (hypothetical)
        // higher-priority Java symbol misses, so the MultiQuery falls
        // through.
        let _h = beans
            .registries
            .jvm_types
            .register(JvmTypeKey::new("com.example.Service"), kotlin_id);

        let queries = vec![
            #[cfg(feature = "java")]
            RegistryQuery::JavaSymbol(JavaSymbolKey::new("com.example.Service")),
            RegistryQuery::JvmType(JvmTypeKey::new("com.example.Service")),
        ];
        let mq = MultiQuery::new(&beans, queries);
        assert_eq!(mq.query(&beans).first(), Some(kotlin_id));
    }

    #[test]
    fn multi_query_observes_provider_change() {
        // The crucial Tier-2 contract: a MultiQuery kept across an edit
        // observes the new state without manual invalidate. Auto-fired
        // via the underlying Registry's subscription on register/drop.
        let beans = Beans::new();
        let mq = MultiQuery::new(
            &beans,
            vec![RegistryQuery::JvmType(JvmTypeKey::new(
                "com.example.Service",
            ))],
        );
        assert!(mq.query(&beans).is_empty());

        let new_id = NodeId::placeholder(42);
        let _h = beans
            .registries
            .jvm_types
            .register(JvmTypeKey::new("com.example.Service"), new_id);

        // Without manual invalidate, the cache flipped to Stale via the
        // wrapping callback; next query recomputes.
        match mq.query(&beans) {
            QueryResult::One(id) => assert_eq!(id, new_id),
            other => panic!("expected One after registration, got {:?}", other),
        }
    }

    #[test]
    fn multi_query_subscribe_fires_on_underlying_change() {
        // Consumer subscribes to the MultiQuery; underlying registry
        // mutates; consumer's callback fires.
        let beans = Beans::new();
        let mq = MultiQuery::new(
            &beans,
            vec![RegistryQuery::JvmType(JvmTypeKey::new(
                "com.example.Service",
            ))],
        );

        let counter = Rc::new(Cell::new(0u32));
        let cb_counter = Rc::clone(&counter);
        let _watch = mq.subscribe(Rc::new(move || {
            cb_counter.set(cb_counter.get() + 1);
        }));

        let _h = beans
            .registries
            .jvm_types
            .register(JvmTypeKey::new("com.example.Service"), NodeId::placeholder(1));
        assert!(counter.get() > 0, "consumer's callback should fire");
    }

    #[test]
    fn multi_query_subscribe_drop_stops_notifications() {
        let beans = Beans::new();
        let mq = MultiQuery::new(
            &beans,
            vec![RegistryQuery::JvmType(JvmTypeKey::new(
                "com.example.Service",
            ))],
        );

        let counter = Rc::new(Cell::new(0u32));
        let cb_counter = Rc::clone(&counter);
        let watch = mq.subscribe(Rc::new(move || {
            cb_counter.set(cb_counter.get() + 1);
        }));

        let _h = beans
            .registries
            .jvm_types
            .register(JvmTypeKey::new("com.example.Service"), NodeId::placeholder(1));
        let after_first = counter.get();
        assert!(after_first > 0);

        drop(watch);

        // Another mutation. Counter must not advance — subscriber is dropped.
        let _h2 = beans
            .registries
            .jvm_types
            .register(JvmTypeKey::new("com.example.Service"), NodeId::placeholder(2));
        assert_eq!(counter.get(), after_first, "dropped handle stops notifications");
    }
}
