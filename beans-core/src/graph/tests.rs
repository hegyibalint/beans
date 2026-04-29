//! Integration tests for the graph engine skeleton.
//!
//! `TestNode` is a minimal payload that exercises the engine surface:
//! it carries a name, an optional `ProviderHandle`, an optional
//! `SubscriptionHandle`, and a counter that a callback can bump. The
//! lifecycle is wired manually here (no `NodeBehavior` impl) so each test
//! is a tight, transparent script.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::graph::arena::{Graph, NodeId};
use crate::graph::cache_state::{CacheState, Generation};
use crate::graph::registry::{ProviderHandle, Registry, SubscriptionHandle};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestKey(&'static str);

struct TestNode {
    name: &'static str,
    provider: Option<ProviderHandle<TestKey>>,
    subscription: Option<SubscriptionHandle<TestKey>>,
    notifications: Rc<Cell<u32>>,
}

impl TestNode {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            provider: None,
            subscription: None,
            notifications: Rc::new(Cell::new(0)),
        }
    }
}

#[test]
fn insert_and_lookup() {
    let mut graph: Graph<TestNode> = Graph::new();
    let id = graph.insert(TestNode::new("alpha"), None);

    let node = graph.get(id).expect("node should exist");
    assert_eq!(node.payload.name, "alpha");
    assert_eq!(node.parent, None);
    assert!(node.children.is_empty());

    // NodeId stable within session — two reads return the same slot.
    assert_eq!(graph.get(id).unwrap().payload.name, "alpha");
    assert_eq!(id, NodeId(0));
}

#[test]
fn hard_link_cascade() {
    let mut graph: Graph<TestNode> = Graph::new();
    let parent = graph.insert(TestNode::new("parent"), None);
    let child = graph.insert(TestNode::new("child"), Some(parent));
    let grandchild = graph.insert(TestNode::new("grandchild"), Some(child));

    assert_eq!(graph.get(parent).unwrap().children, vec![child]);
    assert_eq!(graph.get(child).unwrap().children, vec![grandchild]);
    assert_eq!(graph.get(grandchild).unwrap().parent, Some(child));

    graph.destroy(parent);

    assert!(!graph.contains(parent));
    assert!(!graph.contains(child));
    assert!(!graph.contains(grandchild));
}

#[test]
fn provider_handle_drop_unregisters() {
    let mut graph: Graph<TestNode> = Graph::new();
    let registry: Registry<TestKey> = Registry::new();

    let id = graph.insert(TestNode::new("provider-node"), None);
    let key = TestKey("foo");
    {
        let node = graph.get_mut(id).unwrap();
        node.payload.provider = Some(registry.register(key.clone(), id));
    }

    assert_eq!(registry.query(&key), vec![id]);

    graph.destroy(id);

    assert!(registry.query(&key).is_empty());
}

#[test]
fn subscription_handle_drop_unsubscribes() {
    let mut graph: Graph<TestNode> = Graph::new();
    let registry: Registry<TestKey> = Registry::new();
    let key = TestKey("watch-me");

    let id = graph.insert(TestNode::new("subscriber"), None);
    let counter = graph.get(id).unwrap().payload.notifications.clone();
    {
        let cb_counter = counter.clone();
        let cb = Rc::new(move || {
            cb_counter.set(cb_counter.get() + 1);
        });
        let node = graph.get_mut(id).unwrap();
        node.payload.subscription = Some(registry.subscribe(key.clone(), cb));
    }

    // Sanity: callback fires while subscribed.
    registry.notify(&key);
    assert_eq!(counter.get(), 1);

    graph.destroy(id);

    // Counter is dropped along with the node, but the registry should no
    // longer carry the callback. Notifying again must not panic.
    registry.notify(&key);
    // The strong reference inside the now-dropped Callback is gone; counter
    // still has one strong handle (this test owns it) and reads zero
    // additional increments.
    assert_eq!(counter.get(), 1);
}

#[test]
fn notification_fires() {
    let registry: Registry<TestKey> = Registry::new();
    let key = TestKey("fanout");

    let counter = Rc::new(Cell::new(0u32));
    let cb_counter = counter.clone();
    let _sub = registry.subscribe(
        key.clone(),
        Rc::new(move || {
            cb_counter.set(cb_counter.get() + 1);
        }),
    );

    // Provider registration alone does not notify; a later `notify` does.
    let _provider = registry.register(key.clone(), NodeId(42));
    assert_eq!(counter.get(), 0);

    registry.notify(&key);
    assert_eq!(counter.get(), 1);

    registry.notify(&key);
    assert_eq!(counter.get(), 2);
}

#[test]
fn snapshot_and_release_allows_reentrant_query() {
    // A subscriber callback that reads the providers map on the same
    // registry must not RefCell-panic. The snapshot-and-release pattern
    // copies the callback list out under a short borrow and drops the
    // borrow before invoking, so the callback is free to re-enter.
    let registry: Registry<TestKey> = Registry::new();
    let key = TestKey("reentrant");

    let _provider_a = registry.register(key.clone(), NodeId(1));
    let _provider_b = registry.register(key.clone(), NodeId(2));

    let observed: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let observed_in_cb = observed.clone();
    let registry_in_cb = registry.clone();
    let key_in_cb = key.clone();
    let _sub = registry.subscribe(
        key.clone(),
        Rc::new(move || {
            // Re-enter: query the same registry from inside the callback.
            let providers = registry_in_cb.query(&key_in_cb);
            observed_in_cb.set(providers.len());
        }),
    );

    registry.notify(&key);

    assert_eq!(observed.get(), 2);
}

#[test]
fn free_list_reuses_slots() {
    let mut graph: Graph<TestNode> = Graph::new();

    let a = graph.insert(TestNode::new("a"), None);
    let b = graph.insert(TestNode::new("b"), None);
    let c = graph.insert(TestNode::new("c"), None);

    assert_eq!(a, NodeId(0));
    assert_eq!(b, NodeId(1));
    assert_eq!(c, NodeId(2));

    graph.destroy(b);
    assert!(!graph.contains(b));

    // The next insert should land in slot 1, the freed one.
    let d = graph.insert(TestNode::new("d"), None);
    assert_eq!(d, NodeId(1));
    assert_eq!(graph.get(d).unwrap().payload.name, "d");

    // Original slots a and c are untouched.
    assert_eq!(graph.get(a).unwrap().payload.name, "a");
    assert_eq!(graph.get(c).unwrap().payload.name, "c");
}

#[test]
fn generation_is_monotonic() {
    let mut graph: Graph<TestNode> = Graph::new();
    let id = graph.insert(TestNode::new("gen"), None);

    let g0 = graph.current_generation();

    graph.mark_stale(id);
    let g1 = graph.current_generation();
    assert!(g1 > g0);
    assert_eq!(graph.get(id).unwrap().state, CacheState::Stale);

    graph.mark_fresh(id, g1);
    assert_eq!(graph.get(id).unwrap().state, CacheState::Fresh(g1));
    // mark_fresh does not touch the global counter.
    assert_eq!(graph.current_generation(), g1);

    graph.mark_stale(id);
    let g2 = graph.current_generation();
    assert!(g2 > g1);

    graph.mark_stale(id);
    let g3 = graph.current_generation();
    assert!(g3 > g2);
}

#[test]
fn destroyed_subtree_detaches_from_parent() {
    // Hard-link cascade plus parent fix-up: after destroying a child, the
    // parent's `children` list no longer references it.
    let mut graph: Graph<TestNode> = Graph::new();
    let parent = graph.insert(TestNode::new("p"), None);
    let child_a = graph.insert(TestNode::new("a"), Some(parent));
    let child_b = graph.insert(TestNode::new("b"), Some(parent));

    assert_eq!(graph.get(parent).unwrap().children, vec![child_a, child_b]);

    graph.destroy(child_a);

    assert!(graph.contains(parent));
    assert!(!graph.contains(child_a));
    assert_eq!(graph.get(parent).unwrap().children, vec![child_b]);
}

#[test]
fn registry_outliving_handles_does_not_panic() {
    // Drop order: registry first, then handles. Per ADR-0015 the handles'
    // Drop should no-op via failed Weak::upgrade rather than panic.
    let registry: Registry<TestKey> = Registry::new();
    let key = TestKey("drop-order");

    let provider = registry.register(key.clone(), NodeId(7));
    let counter = Rc::new(Cell::new(0u32));
    let cb_counter = counter.clone();
    let subscription = registry.subscribe(
        key.clone(),
        Rc::new(move || cb_counter.set(cb_counter.get() + 1)),
    );

    drop(registry);
    // These drops happen here; if the Weak upgrade did not gracefully
    // no-op we would either panic or borrow-mut a dangling cell.
    drop(provider);
    drop(subscription);

    assert_eq!(counter.get(), 0);
}

#[test]
fn generation_zero_default() {
    let graph: Graph<TestNode> = Graph::new();
    assert_eq!(graph.current_generation(), Generation::ZERO);
}

#[test]
fn duplicate_provider_registration_drops_one_at_a_time() {
    // RAII invariant: each ProviderHandle owns exactly one entry. If a node
    // registers twice for the same key, dropping one handle must leave the
    // other entry intact.
    let registry: Registry<TestKey> = Registry::new();
    let key = TestKey("dup");
    let node = NodeId(7);

    let h1 = registry.register(key.clone(), node);
    let h2 = registry.register(key.clone(), node);

    assert_eq!(registry.query(&key), vec![node, node]);

    drop(h1);
    assert_eq!(registry.query(&key), vec![node]);

    drop(h2);
    assert!(registry.query(&key).is_empty());
}

// --------- Dynamic-link tests (ADR-0006, ADR-0008). ---------
//
// `TwoRegistryCtx` plays the role of a real `Registries` struct: it owns
// two registries with different keys (`TestKey` and `OtherKey`) so the
// query enum has to dispatch to the right one. This is the smallest
// faithful model of the typed-key discipline ADR-0012 commits to — a
// single test registry would let us cheat by reusing one key type
// everywhere.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OtherKey(&'static str);

struct TwoRegistryCtx {
    primary: Registry<TestKey>,
    secondary: Registry<OtherKey>,
}

#[derive(Debug, Clone)]
enum TestQuery {
    Primary(TestKey),
    Secondary(OtherKey),
}

impl crate::graph::dynamic_link::RegistryQuery for TestQuery {
    type Ctx = TwoRegistryCtx;

    fn resolve(&self, ctx: &Self::Ctx) -> Vec<NodeId> {
        match self {
            TestQuery::Primary(k) => ctx.primary.query(k),
            TestQuery::Secondary(k) => ctx.secondary.query(k),
        }
    }
}

#[test]
fn dynamic_link_first_match_picks_first_hit() {
    use crate::graph::dynamic_link::DynamicLink;

    let ctx = TwoRegistryCtx {
        primary: Registry::new(),
        secondary: Registry::new(),
    };

    // Only the *secondary* key has a provider — the primary query misses
    // and the link falls through.
    let _h = ctx.secondary.register(OtherKey("svc"), NodeId(42));

    let mut link = DynamicLink::first_match(vec![
        TestQuery::Primary(TestKey("svc")),
        TestQuery::Secondary(OtherKey("svc")),
    ]);

    assert_eq!(link.resolve(&ctx), Some(NodeId(42)));
    assert_eq!(link.active_index(), Some(1));
    assert_eq!(link.cached_result(), Some(NodeId(42)));
}

#[test]
fn dynamic_link_first_match_prefers_higher_priority() {
    use crate::graph::dynamic_link::DynamicLink;

    let ctx = TwoRegistryCtx {
        primary: Registry::new(),
        secondary: Registry::new(),
    };

    // Both registries hold the symbol; the primary wins because it is
    // listed first.
    let _hp = ctx.primary.register(TestKey("svc"), NodeId(7));
    let _hs = ctx.secondary.register(OtherKey("svc"), NodeId(99));

    let mut link = DynamicLink::first_match(vec![
        TestQuery::Primary(TestKey("svc")),
        TestQuery::Secondary(OtherKey("svc")),
    ]);

    assert_eq!(link.resolve(&ctx), Some(NodeId(7)));
    assert_eq!(link.active_index(), Some(0));
}

#[test]
fn dynamic_link_resolve_returns_none_when_all_queries_miss() {
    use crate::graph::dynamic_link::DynamicLink;

    let ctx = TwoRegistryCtx {
        primary: Registry::new(),
        secondary: Registry::new(),
    };

    let mut link = DynamicLink::first_match(vec![
        TestQuery::Primary(TestKey("missing")),
        TestQuery::Secondary(OtherKey("missing")),
    ]);

    assert_eq!(link.resolve(&ctx), None);
    assert_eq!(link.active_index(), None);
    assert_eq!(link.cached_result(), None);
}

#[test]
fn dynamic_link_invalidate_clears_cache() {
    use crate::graph::dynamic_link::DynamicLink;

    let ctx = TwoRegistryCtx {
        primary: Registry::new(),
        secondary: Registry::new(),
    };
    let _h = ctx.primary.register(TestKey("svc"), NodeId(1));

    let mut link = DynamicLink::first_match(vec![TestQuery::Primary(TestKey("svc"))]);

    assert_eq!(link.resolve(&ctx), Some(NodeId(1)));
    assert_eq!(link.cached_result(), Some(NodeId(1)));

    link.invalidate();
    assert_eq!(link.cached_result(), None);
    assert_eq!(link.active_index(), None);
}

#[test]
fn dynamic_link_falls_through_when_higher_priority_misses() {
    use crate::graph::dynamic_link::DynamicLink;

    let ctx = TwoRegistryCtx {
        primary: Registry::new(),
        secondary: Registry::new(),
    };

    // Start with only the secondary registered.
    let hs = ctx.secondary.register(OtherKey("svc"), NodeId(99));

    let mut link = DynamicLink::first_match(vec![
        TestQuery::Primary(TestKey("svc")),
        TestQuery::Secondary(OtherKey("svc")),
    ]);

    // First resolve falls through to the secondary.
    assert_eq!(link.resolve(&ctx), Some(NodeId(99)));
    assert_eq!(link.active_index(), Some(1));

    // A higher-priority provider appears.
    let _hp = ctx.primary.register(TestKey("svc"), NodeId(7));
    link.invalidate();
    assert_eq!(link.resolve(&ctx), Some(NodeId(7)));
    assert_eq!(link.active_index(), Some(0));

    // Higher-priority provider goes away again.
    drop(_hp);
    link.invalidate();
    assert_eq!(link.resolve(&ctx), Some(NodeId(99)));
    assert_eq!(link.active_index(), Some(1));

    // Last provider goes away too.
    drop(hs);
    link.invalidate();
    assert_eq!(link.resolve(&ctx), None);
}

#[test]
fn dynamic_link_merge_all_unions_results_in_query_order() {
    use crate::graph::dynamic_link::DynamicLink;

    let ctx = TwoRegistryCtx {
        primary: Registry::new(),
        secondary: Registry::new(),
    };

    // Two providers in primary, one in secondary. MergeAll returns all
    // three, primary first (queries are walked in order).
    let _hp1 = ctx.primary.register(TestKey("svc"), NodeId(1));
    let _hp2 = ctx.primary.register(TestKey("svc"), NodeId(2));
    let _hs = ctx.secondary.register(OtherKey("svc"), NodeId(99));

    let link = DynamicLink::merge_all(vec![
        TestQuery::Primary(TestKey("svc")),
        TestQuery::Secondary(OtherKey("svc")),
    ]);

    assert_eq!(
        link.resolve_all(&ctx),
        vec![NodeId(1), NodeId(2), NodeId(99)]
    );
}

#[test]
fn dynamic_link_resolve_caches_first_provider_when_query_has_many() {
    use crate::graph::dynamic_link::DynamicLink;

    let ctx = TwoRegistryCtx {
        primary: Registry::new(),
        secondary: Registry::new(),
    };

    // Two providers for the same key — `resolve` (FirstMatch single) takes
    // the first one. Per ADR-0013 the registry's provider order has no
    // semantic weight, so callers needing precedence among multiple hits
    // in one registry must encode that as additional queries.
    let _h1 = ctx.primary.register(TestKey("svc"), NodeId(1));
    let _h2 = ctx.primary.register(TestKey("svc"), NodeId(2));

    let mut link = DynamicLink::first_match(vec![TestQuery::Primary(TestKey("svc"))]);
    assert_eq!(link.resolve(&ctx), Some(NodeId(1)));
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
    let _sub = primary.subscribe(
        key_a.clone(),
        Rc::new(move || {
            // Re-enter the *secondary* registry from inside the primary's
            // notification path.
            let h = secondary_in_cb.register(key_b_in_cb.clone(), NodeId(123));
            *dh_in_cb.borrow_mut() = Some(h);
        }),
    );

    // Before notify: secondary is empty.
    assert!(secondary.query(&key_b).is_empty());

    primary.notify(&key_a);

    // After notify: secondary has the provider the callback registered,
    // and the primary's internals are still usable (re-entrancy didn't
    // wedge it).
    assert_eq!(secondary.query(&key_b), vec![NodeId(123)]);
    let _ = primary.register(key_a.clone(), NodeId(7));

    // Drop the derived handle — secondary cleans up.
    *derived_handle.borrow_mut() = None;
    assert!(secondary.query(&key_b).is_empty());
}

#[test]
fn dynamic_link_resolve_re_walks_registries_after_invalidate() {
    use crate::graph::dynamic_link::DynamicLink;

    // Stability check: a link that resolved against one provider should,
    // after invalidate(), pick up a new higher-priority provider that
    // appeared in the meantime. Documents the contract: callers MUST
    // call `invalidate` to rebuild the cache; the link does not poll on
    // its own (subscription tiering — ADR-0008 — would handle that and
    // is deferred per backlog #027).
    let ctx = TwoRegistryCtx {
        primary: Registry::new(),
        secondary: Registry::new(),
    };
    let _hs = ctx.secondary.register(OtherKey("svc"), NodeId(99));

    let mut link = DynamicLink::first_match(vec![
        TestQuery::Primary(TestKey("svc")),
        TestQuery::Secondary(OtherKey("svc")),
    ]);
    assert_eq!(link.resolve(&ctx), Some(NodeId(99)));
    assert_eq!(link.active_index(), Some(1));

    // Higher-priority provider appears. Without invalidate the link's
    // cached state is *stale* — by design until subscriptions land.
    let _hp = ctx.primary.register(TestKey("svc"), NodeId(7));
    assert_eq!(link.cached_result(), Some(NodeId(99)));

    // After invalidate the next resolve picks up the new provider.
    link.invalidate();
    assert_eq!(link.resolve(&ctx), Some(NodeId(7)));
    assert_eq!(link.active_index(), Some(0));
}
