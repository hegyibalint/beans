# ADR-0014: Use RAII handles for subscriptions and provider registrations

## Status

Accepted

## Context

A node in the graph is both a **provider** (it registers itself in
zero or more registries) and a **subscriber** (it watches zero or more
registry keys to be notified when something changes). When the node
dies — file deleted, parse failed, replaced by a new version — every
provider entry it owns and every subscription it placed must be cleaned
up. Forgetting either side is a real bug:

- An orphaned provider entry causes resolution to return a `NodeId`
  pointing at freed memory or a re-allocated unrelated node.
- An orphaned subscription causes notifications to be sent to a node
  that no longer exists, or to fan out forever as the subscriber list
  grows.

The naive shape — explicit `subscribe`/`unsubscribe` pairs returning
opaque `SubscriptionId` values, the caller carrying those IDs around
and remembering to call `unsubscribe` in `on_destroyed` — works in
principle. In practice it is a perpetual source of leaks. Every code
path that drops a subscriber early (error path, partial parse, GC walk
that bails halfway) is a chance to forget the cleanup.

The graph's destruction is also recursive: when a file is deleted, a
walk over the hard-link tree drops dozens of nodes in one operation.
Each node has one to three provider registrations and zero to many
subscriptions. Doing all of that cleanup correctly with manual
bookkeeping is the kind of thing that mostly works for a year and then
leaks for a week.

## Decision

Subscriptions and provider registrations are returned as **owning
handles** whose `Drop` impl performs the cleanup:

```rust
struct ProviderHandle {
    // back-reference to the registry, key, node id (see ADR-0015)
}

impl Drop for ProviderHandle {
    fn drop(&mut self) {
        // remove this provider entry, notify subscribers if needed
    }
}

struct SubscriptionHandle { /* ... */ }
impl Drop for SubscriptionHandle { /* ... */ }
```

A `NodeData` holds its handles in plain `Vec`s:

```rust
struct NodeData {
    // ...
    providers: Vec<ProviderHandle>,
    subscriptions: Vec<SubscriptionHandle>,
}
```

When a `NodeData` is dropped — for any reason, on any path — the `Vec`
drops, each handle drops, and each handle's `Drop` impl removes the
corresponding registry entry. There is no separate `on_destroyed`
cleanup step that the graph is responsible for calling.

Explicit cancellation methods (`SubscriptionHandle::cancel(self)`) may
be exposed where early cancellation is useful (e.g., a subscription
that becomes irrelevant before the parent node is dropped). RAII via
`Drop` is the default; `cancel` is an optimization for specific cases.

## Consequences

**Positive.**

- Cleanup is structural. There is no path that drops a node without
  cleaning up its registry entries — the borrow checker does not let us
  drop a `NodeData` without dropping its `Vec<ProviderHandle>`.
- Partial-construction failures clean up correctly. If a node creation
  succeeds in registering 2 of 3 providers and then fails, the 2 that
  did register are dropped on the way out.
- The "registration order is reverse of destruction order" property
  comes for free from `Vec` drop order. We do not have to think about
  it.
- Code that creates and tears down nodes (tests, fixtures, the GC walk)
  is shorter and clearer. There is no "and now unregister everything"
  pass.

**Negative.**

- Handles need a back-reference to the registry, which has lifetime
  implications (handled in ADR-0015). The shape is not free.
- Drop runs at unpredictable times. A subscription that fires a
  notification on cleanup runs that notification during `Drop`, which
  is a constrained context — no panicking, careful with re-entrant
  borrows. We address re-entrancy with the snapshot-and-release pattern
  (ADR-0015).
- Errors during cleanup cannot be returned. If the registry's interior
  state is somehow corrupt, `Drop` cannot signal a failure to the
  caller. We treat this as a panic-worthy bug, not a recoverable
  situation.
- Long-lived subscriptions that intentionally outlive their nodes (rare,
  but possible — e.g., a user-facing watch in the LSP layer) cannot
  live in a `NodeData`. They live in whatever owns the LSP request,
  which is fine and matches their actual lifetime.

## Alternatives considered

**Explicit `subscribe`/`unsubscribe` calls with `SubscriptionId`
values.** Caller stores the ID, calls `unsubscribe` in `on_destroyed`.
Rejected because every code path that drops a node has to remember to
call it. We have many such paths (parse failure, GC, replacement,
test cleanup) and forgetting on any one of them leaks.

**Centralized lifetime tracker that walks the graph and reaps orphans
periodically.** Rejected because it is a workaround for not having
RAII. The cost (background work, heuristics about orphan detection,
the window during which an orphan exists) is greater than the cost of
holding handles.

**Handles that require explicit `cancel()` and panic on `Drop` if not
cancelled.** Forces the caller to acknowledge cleanup. Rejected
because the natural place for cleanup — when a node is being destroyed
— is exactly the place where `Drop` runs anyway. Forcing an explicit
call adds ceremony without adding safety.

**Bundle subscriptions and providers into a single typed
`NodeRegistration` aggregate.** A single owning struct that registers
on creation and unregisters on `Drop`. Considered briefly. Rejected
because nodes have variable, dynamic registration counts (a Java class
registers in one registry, a Kotlin extension function registers in
two or three), and a typed aggregate would have to be either generic
over all combinations or accept a `Vec` internally — at which point we
are back to plain handles in a `Vec`.
