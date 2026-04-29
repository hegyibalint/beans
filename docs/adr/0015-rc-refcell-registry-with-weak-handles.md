# ADR-0015: Registries are `Rc<RefCell<_>>` with `Weak` back-references in handles

## Status

Accepted

## Context

ADR-0014 commits us to RAII handles whose `Drop` impl cleans up the
corresponding registry entries. That implies handles need a way to find
the registry. The straightforward shapes are:

- **Borrowed reference (`&'a Registry`).** Handles carry a lifetime;
  every node, every subscription, every layer that holds handles is
  parameterized by `'a`. The lifetime infects the entire graph.
- **`'static` singleton.** Make the registry a global. No lifetimes,
  but global mutable state, no isolation between graph instances (e.g.,
  multiple workspaces in one process), terrible for tests.
- **`Arc<Mutex<_>>` shared ownership.** No lifetimes, multi-threaded.
  But the graph is single-threaded (ADR-0018), and `Mutex` for an
  inherently single-threaded data structure is wasted overhead and
  spurious deadlock risk.

The single-threaded constraint (ADR-0018) and the multi-instance
requirement (each LSP workspace gets its own graph) point at
`Rc<RefCell<_>>`: shared ownership without thread-safety overhead, with
interior mutability for the registry's state.

Handles back-referencing the registry is more nuanced. If a handle
holds an `Rc<RefCell<RegistryInner>>`, then while any handle exists
the registry is kept alive. Tearing down a graph would require dropping
every handle before the registry, and any cycle (registry holds nodes,
nodes hold handles, handles hold registry) leaks. Concretely, the
registry's subscriber list contains `NodeId`-keyed entries that point
back at nodes that own subscriptions — a real cycle.

The other concern is **re-entrancy under `RefCell`**. A notification
fires inside `borrow_mut()` of the registry. The notification calls a
subscriber's callback, which queries the registry, which calls
`borrow()` — and `RefCell` panics. This is not theoretical; it is the
default shape of every event-fan-out system unless precautions are
taken.

## Decision

The registry is shaped as:

```rust
struct Registry<K> {
    inner: Rc<RefCell<RegistryInner<K>>>,
}

struct RegistryInner<K> {
    providers: HashMap<K, Vec<NodeId>>,
    subscribers: HashMap<K, Vec<SubscriberEntry>>,
}
```

Handles back-reference the registry via **`Weak`**:

```rust
struct ProviderHandle<K> {
    registry: Weak<RefCell<RegistryInner<K>>>,
    key: K,
    node: NodeId,
}

impl<K> Drop for ProviderHandle<K> {
    fn drop(&mut self) {
        if let Some(reg) = self.registry.upgrade() {
            reg.borrow_mut().remove_provider(&self.key, self.node);
        }
        // If the registry is already gone, nothing to clean up — the
        // entire registry has been dropped, including the entry this
        // handle pointed at.
    }
}
```

This means handles do **not** keep the registry alive. When the whole
graph is being torn down (e.g., workspace shutdown), the registry can
drop while handles still exist; their drops then no-op via the failed
`upgrade()`. No leaks, no double-free, no required ordering.

Notifications follow the **snapshot-and-release** pattern:

```rust
fn notify(&self, key: &K) {
    // 1. Borrow, copy out the relevant subscriber list, release the borrow.
    let subscribers: Vec<Callback> = {
        let inner = self.inner.borrow();
        inner.subscribers.get(key).cloned().unwrap_or_default()
    };
    // 2. Fire callbacks with no borrow held. Callbacks may re-enter
    //    the registry safely.
    for cb in subscribers {
        cb.run();
    }
}
```

Subscriber callbacks may freely call back into the registry — adding
or removing subscriptions, querying, registering — because no borrow
is held at the point of dispatch. Any modifications they make take
effect on the next notification cycle.

## Consequences

**Positive.**

- Tear-down is robust. Dropping the registry first does not corrupt
  outstanding handles; their `Drop` impls degrade gracefully via the
  failed `Weak::upgrade`.
- No reference cycles. The registry holds no strong references to
  handles; handles hold weak references to the registry.
- Re-entrant callbacks are safe. The snapshot-and-release pattern
  removes the most common source of `RefCell` panics in event-driven
  systems.
- Single-threaded `RefCell` is cheap — no atomics, no contention, no
  lock-fairness concerns. Borrow violations panic immediately, which
  surfaces bugs at the point of the bug rather than as data corruption
  later.

**Negative.**

- Every handle drop pays an `upgrade()` plus a possible `borrow_mut()`.
  In aggregate this is real cost when tearing down large subtrees. We
  have measured it; it is dwarfed by the cost of dropping the actual
  node values. If that ever stops being true, batch-drop paths (drop
  a `Vec<ProviderHandle>` against the registry once, instead of one
  by one) are the obvious optimization.
- The snapshot-and-release pattern requires copying (`Vec<Callback>`)
  on every notification. For keys with hundreds of subscribers this
  is a real per-notification cost. We accept it; correctness over a
  few microseconds.
- A subscriber added during a callback does not see that same
  notification. This is the price of releasing the borrow before
  dispatching. We document it; the alternative (re-checking the
  subscriber list during dispatch) reintroduces the panic risk.
- Panics from a misbehaving callback unwind through `notify`. The LSP
  layer wraps request handlers in `catch_unwind` (ADR-0018) so a panic
  in one callback does not kill the server, but other callbacks for
  the same notification do not run.

## Alternatives considered

**Borrowed `&'a Registry` references in handles.** Pure Rust, no
runtime cost, no `Weak`. Rejected because the lifetime `'a` would
infect every type that owns a handle (every `NodeData`, every
`Vec<NodeData>`, every layer that owns nodes). The graph is
self-referential at multiple levels and lifetimes do not compose cleanly
across self-references.

**`'static` singleton registry.** No lifetimes, no `Weak`, simple. But
multiple LSP workspaces share a process; each needs its own isolated
graph. A singleton registry forces a single graph per process or a
`HashMap<WorkspaceId, RegistryInner>` indirection, neither of which we
want. Rejected.

**`Arc<Mutex<_>>` for thread-safety.** Allows multi-threaded mutation.
Rejected per ADR-0018 — the graph is single-threaded by design, and
`Mutex` adds atomic costs we do not need plus a deadlock surface we
do not want.

**Strong `Rc` back-references plus explicit "cleanup before drop"
discipline.** Handles hold `Rc`; the graph is responsible for dropping
all handles before dropping the registry. Rejected because it depends
on a discipline we cannot enforce. Any code path that drops the
registry first leaks. RAII is meant to remove this kind of fragility,
and `Weak` keeps that property.

**Subscriber callbacks queued and dispatched on a separate "tick"
boundary** (instead of snapshot-and-release inline). Avoids any
re-entrancy concerns by deferring all dispatch. Rejected because it
adds an explicit phase boundary that resolution code has to know about
(call notify, then call tick, then check results). Snapshot-and-release
inline keeps the API simple and pushes the cost into the registry.
