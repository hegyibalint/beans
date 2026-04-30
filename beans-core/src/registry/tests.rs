//! Registry-layer tests.
//!
//! Two flavours:
//!
//! * **Pure registry**: provider/subscription lifecycle, RAII drop
//!   semantics, snapshot-and-release re-entrancy, auto-notify on
//!   register/drop. No `Graph` needed; `NodeId(u64)` literals stand in
//!   for graph slots.
//! * **Graph-integrated**: a `TestNode` payload holds a real
//!   `ProviderHandle` / `SubscriptionHandle`; destroying the node drops
//!   the handles, and the test asserts on registry state. These pin the
//!   contract that makes registries useful with the graph: when the
//!   graph frees a node, its handles' `Drop` impls clean up the registry.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::graph::{Graph, NodeId};
use crate::registry::{ProviderHandle, Registry, SubscriptionHandle};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TestKey(&'static str);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OtherKey(&'static str);

/// Minimal payload used by graph-integrated registry tests.
struct HandleNode {
    provider: Option<ProviderHandle<TestKey>>,
    subscription: Option<SubscriptionHandle<TestKey>>,
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
    // node drops the handle, which removes the registry entry. End-to-end
    // RAII per ADR-0014.
    let mut graph: Graph<HandleNode> = Graph::new();
    let registry: Registry<TestKey> = Registry::new();

    let id = graph.insert(HandleNode::new(), None);
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
    // Graph-integrated mirror: a node holds a SubscriptionHandle.
    // Destroying the node drops it; later notifies do not reach the
    // (now-dropped) callback.
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
        node.payload.subscription = Some(registry.subscribe(key.clone(), cb));
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
    let _sub = registry.subscribe(
        key.clone(),
        Rc::new(move || {
            cb_counter.set(cb_counter.get() + 1);
        }),
    );

    let provider = registry.register(key.clone(), NodeId(42));
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
