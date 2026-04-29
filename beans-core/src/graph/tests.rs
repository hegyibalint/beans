//! Integration tests for the graph engine skeleton.
//!
//! `TestNode` is a minimal payload that exercises the engine surface:
//! it carries a name, an optional `ProviderHandle`, an optional
//! `SubscriptionHandle`, and a counter that a callback can bump. The
//! lifecycle is wired manually here (no `NodeBehavior` impl) so each test
//! is a tight, transparent script.

use std::cell::Cell;
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
