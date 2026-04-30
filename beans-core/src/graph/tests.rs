//! Graph-engine tests: arena lifecycle (insert / destroy / hard-link
//! cascade / free-list reuse / generation), and dynamic-link mechanics
//! (which use [`Registry`] as the example resolver because
//! [`RegistryQuery`] is the trait every cross-file lookup goes through).
//!
//! Pure registry tests (provider/subscription RAII, snapshot-and-release,
//! auto-notify on register/drop) live with their owner in
//! [`crate::registry`].

use crate::graph::arena::{Graph, NodeId};
use crate::graph::cache_state::{CacheState, Generation};
use crate::registry::Registry;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestKey(&'static str);

struct TestNode {
    name: &'static str,
}

impl TestNode {
    fn new(name: &'static str) -> Self {
        Self { name }
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
fn generation_zero_default() {
    let graph: Graph<TestNode> = Graph::new();
    assert_eq!(graph.current_generation(), Generation::ZERO);
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

#[test]
fn dynamic_link_resolve_short_circuits_through_cache() {
    use crate::graph::dynamic_link::DynamicLink;

    // Per the documented contract: once `resolve` populates the cache,
    // subsequent calls return the cached value without re-walking
    // queries. We verify this by mutating the registry the link would
    // otherwise consult — it should *not* observe the change without an
    // explicit `invalidate`. Subscription-driven auto-invalidation is
    // the eventual answer (backlog #027); until then, callers own the
    // freshness contract.
    let ctx = TwoRegistryCtx {
        primary: Registry::new(),
        secondary: Registry::new(),
    };
    let _h = ctx.primary.register(TestKey("svc"), NodeId(1));

    let mut link = DynamicLink::first_match(vec![
        TestQuery::Primary(TestKey("svc")),
        TestQuery::Secondary(OtherKey("svc")),
    ]);

    // Prime the cache.
    assert_eq!(link.resolve(&ctx), Some(NodeId(1)));
    assert_eq!(link.cached_result(), Some(NodeId(1)));

    // Drop the only provider. Without invalidate, the next resolve must
    // still return the cached NodeId — even though the registry has
    // moved on. This is the documented "explicit invalidate" contract.
    drop(_h);
    assert!(ctx.primary.query(&TestKey("svc")).is_empty());
    assert_eq!(link.resolve(&ctx), Some(NodeId(1)));

    // After invalidate, the next resolve sees the (now-empty) registry.
    link.invalidate();
    assert_eq!(link.resolve(&ctx), None);
}
