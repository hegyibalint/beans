//! Cross-registry query abstractions — both stateless one-shots and
//! stored, subscription-backed cached lookups.
//!
//! Two layers, sized to caller need:
//!
//! ## Stateless: `Queryable<M>` + `first_match` / `all_matches`
//!
//! [`Queryable<M>`] is the trait registries implement once per query
//! shape they answer. The native case is trivial: each [`Registry<K>`]
//! impls `Queryable<K>` returning a [`QueryResult`] by calling its own
//! `providers`. Cross-registry query *models* (e.g. [`ByFqn`]) are
//! separate types, with one `Queryable<M>` impl per registry that
//! answers the model — translating the model into the registry's
//! native key on the way through.
//!
//! [`QueryResult`] is a tri-state owned value: `None`, `One(NodeId)`, or
//! `Many(Vec<NodeId>)`. The variant tells the consumer the cardinality
//! at a glance; the zero/one cases never allocate a Vec; pattern matches
//! at the call site make the cardinality explicit. Per ADR-0007 the
//! NodeIds inside are generational handles — safe to hold across
//! registry mutations and to dereference later via the graph's
//! generation-validated `get` (returns `None` if the slot was
//! destroyed).
//!
//! Cross-registry consumption uses [`first_match`] / [`all_matches`]
//! over a `&[&dyn Queryable<M>]` of the registries to consult, named
//! by the caller (no hidden routing). This is the priority-list
//! pattern from ADR-0008 — a Java-side caller might consult
//! `[&beans.registries.java_symbols, &beans.registries.jvm_types]` so
//! the language-native answer wins when present.
//!
//! ## Stored, push-based: `MultiQuery`
//!
//! [`MultiQuery`] holds a `Vec<RegistryQuery>` (one variant per
//! registry, closed enum). Subscribes to each underlying registry on
//! construction; underlying provider-set mutations flip the cached
//! answer to `Stale` and fire any consumer subscribers (`MultiQuery`
//! exposes the same `subscribe(cb) -> handle` shape as `Registry`).
//! Reading via [`MultiQuery::query`] returns the cached answer, or
//! recomputes from the underlying queries on `Stale`.
//!
//! Per-query subscription tiering (value-watch on active, existence-
//! watch on higher-priority misses) is a future optimisation; today
//! every consulted registry is subscribed uniformly.

use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

use super::{Callback, Registry};
use crate::Beans;
use crate::graph::{NodeHandle, NodeId};
use crate::jvm::{Fqn, JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey};

#[cfg(feature = "java")]
use crate::languages::java::JavaSymbolKey;

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
    /// `Some(id)` if there is at least one match (`One` or first of
    /// `Many`), `None` otherwise. Convenience for go-to-definition style
    /// callers that want a single representative.
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
// Queryable<M> + ByFqn
// =========================================================================

/// A registry that can answer a query of shape `M`.
///
/// Each [`Registry<K>`] impls `Queryable<K>` for its own key (the native
/// case). Cross-registry models like [`ByFqn`] are answered by every
/// registry that can translate the model into its native key — so a
/// caller can hand a `&[&dyn Queryable<ByFqn>]` to [`first_match`] and
/// the right registries answer with no caller-side dispatch.
pub trait Queryable<M> {
    fn query(&self, model: &M) -> QueryResult;
}

/// Native-key impl: every `Registry<K>` answers queries of its own key.
impl<K: Eq + Hash + Clone> Queryable<K> for Registry<K> {
    fn query(&self, key: &K) -> QueryResult {
        QueryResult::from(self.providers(key))
    }
}

/// Cross-registry "find me anything with this FQN" query model.
/// Multiple registries answer this; each translates the FQN into its
/// own native key.
#[derive(Debug, Clone)]
pub struct ByFqn(pub Fqn);

impl Queryable<ByFqn> for Registry<JvmTypeKey> {
    fn query(&self, m: &ByFqn) -> QueryResult {
        QueryResult::from(self.providers(&JvmTypeKey::new(m.0.clone())))
    }
}

#[cfg(feature = "java")]
impl Queryable<ByFqn> for Registry<JavaSymbolKey> {
    fn query(&self, m: &ByFqn) -> QueryResult {
        QueryResult::from(self.providers(&JavaSymbolKey::new(m.0.clone())))
    }
}

/// First-match across the given registries, in priority order. The
/// caller names which registries to consult and in what order. Returns
/// the first non-empty answer; subsequent registries are not consulted.
///
/// Resolution-style entry point (go-to-definition, type-checking).
/// Callers that want every match across every consulted registry use
/// [`all_matches`].
pub fn first_match<M>(model: &M, consult: &[&dyn Queryable<M>]) -> Option<NodeId> {
    consult.iter().find_map(|r| r.query(model).first())
}

/// Every match across the given registries, in query order, no dedup.
/// Per ADR-0008 the cross-registry merge does not de-duplicate — a
/// node registered under multiple keys appears once per hit. Consumers
/// that want one entry per logical symbol (typical for completion)
/// collapse downstream with knowledge of which language wins.
pub fn all_matches<M>(model: &M, consult: &[&dyn Queryable<M>]) -> Vec<NodeId> {
    consult.iter().flat_map(|r| r.query(model).all()).collect()
}

// =========================================================================
// RegistryQuery + MultiQuery
// =========================================================================

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
    pub fn providers(&self, beans: &Beans) -> Vec<NodeId> {
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
    pub fn subscribe(&self, beans: &Beans, cb: Callback) -> Box<dyn NodeHandle> {
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
/// `MultiSubscriptionHandle` returned by `MultiQuery::subscribe` holds
/// an `Rc<Cell<bool>>` shared with this entry; on handle drop the flag
/// flips to `false` and the next fire prunes the entry. Same pattern
/// the underlying `Registry<K>` uses internally for subscriptions.
struct MultiSubscriber {
    callback: Callback,
    alive: Rc<Cell<bool>>,
}

/// RAII subscription handle for `MultiQuery::subscribe`. Drop flips
/// the liveness flag to `false`; the MultiQuery prunes lazily on the
/// next fire.
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
/// Construction subscribes to each underlying registry so any
/// provider-set change auto-invalidates the cache and fires consumer
/// subscribers. After construction, `query` is `&self` (read-only); the
/// cache is invalidated through the subscription callbacks, not through
/// the query call.
pub struct MultiQuery {
    queries: Vec<RegistryQuery>,
    cached: Rc<RefCell<Cached>>,
    /// Subscriptions on each underlying registry. RAII: dropped when the
    /// MultiQuery drops, removing the registry-side entries. Stored as
    /// `Box<dyn NodeHandle>` because each variant of `RegistryQuery`
    /// produces a different `SubscriptionHandle<K>` type.
    _internal_subs: Vec<Box<dyn NodeHandle>>,
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
        let subs: Vec<Box<dyn NodeHandle>> = queries
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
    pub fn providers_all(&self, beans: &Beans) -> Vec<NodeId> {
        self.queries.iter().flat_map(|q| q.providers(beans)).collect()
    }

    /// Subscribe to this MultiQuery's invalidations. The callback fires
    /// after the cache has been flipped to `Stale`, so a subscriber
    /// that re-reads via [`query`](Self::query) on its callback sees
    /// the new value.
    ///
    /// Returns a RAII handle; drop it to stop receiving notifications.
    /// Same shape as `Registry::subscribe`.
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

    // ---- QueryResult / Queryable / first_match / all_matches ----

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

    #[test]
    fn registry_native_queryable() {
        let r: Registry<JvmTypeKey> = Registry::new();
        let key = JvmTypeKey::new("com.example.Foo");

        // Empty: query returns None.
        assert!(matches!(r.query(&key), QueryResult::None));

        let _h = r.register(key.clone(), NodeId::placeholder(1));
        assert!(matches!(r.query(&key), QueryResult::One(_)));

        let _h2 = r.register(key.clone(), NodeId::placeholder(2));
        assert!(matches!(r.query(&key), QueryResult::Many(_)));
    }

    #[test]
    fn first_match_walks_in_priority_order() {
        let r1: Registry<JvmTypeKey> = Registry::new();
        let r2: Registry<JvmTypeKey> = Registry::new();
        let key = JvmTypeKey::new("com.example.Foo");

        let _h2 = r2.register(key.clone(), NodeId::placeholder(99));

        // Only r2 has a provider; first_match falls through.
        let result = first_match::<JvmTypeKey>(&key, &[&r1, &r2]);
        assert_eq!(result, Some(NodeId::placeholder(99)));

        // r1 gets a higher-priority provider.
        let _h1 = r1.register(key.clone(), NodeId::placeholder(7));
        let result = first_match::<JvmTypeKey>(&key, &[&r1, &r2]);
        assert_eq!(result, Some(NodeId::placeholder(7)));
    }

    #[test]
    fn all_matches_concatenates_query_order() {
        let r1: Registry<JvmTypeKey> = Registry::new();
        let r2: Registry<JvmTypeKey> = Registry::new();
        let key = JvmTypeKey::new("com.example.Foo");

        let _h1 = r1.register(key.clone(), NodeId::placeholder(1));
        let _h2 = r2.register(key.clone(), NodeId::placeholder(2));

        let results = all_matches::<JvmTypeKey>(&key, &[&r1, &r2]);
        assert_eq!(
            results,
            vec![NodeId::placeholder(1), NodeId::placeholder(2)]
        );
    }

    // ---- MultiQuery ----

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
            vec![RegistryQuery::JvmType(JvmTypeKey::new(
                "com.example.Service",
            ))],
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
