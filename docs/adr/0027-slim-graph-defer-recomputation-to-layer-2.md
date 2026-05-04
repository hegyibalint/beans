# ADR-0027: Limit the graph to a hard-link forest; lazy recomputation lives in layer-2 consumers

## Status

Accepted.

Supersedes:

- [ADR-0006](0006-hard-links-and-dynamic-links.md) — partial; only the
  dynamic-links-as-graph-edge-field portion. The hard-link half remains in
  force.
- [ADR-0009](0009-push-stale-pull-recompute.md).
- [ADR-0010](0010-lazy-recomputation.md).
- [ADR-0011](0011-stable-vs-volatile-nodes.md).

## Context

ADRs 0006/0009/0010/0011 specified the graph as a typed arena that also
carried lazy-recompute machinery: per-node `CacheState`
(`Fresh(generation) | Stale | Computing`), a global generation counter,
`dynamic_links: Vec<DynamicLink>` on `NodeData`, push-stale propagation
upward through those edges, pull-recompute with cycle detection, and a
graph-level stable-vs-volatile node distinction so LSP subscription handles
survive content-clearing edits.

That design predates any layer-2 consumer. It assumed the graph would be
both the storage substrate *and* the orchestrator of lazy recomputation.
Two facts surfaced as the registry layer matured (ADRs 0012/0013/0014/0015
and the rev-3 reshape of 0008):

1. **Cross-language dependency tracking lives at the use site, not on graph
   edges.** ADR-0008 rev 3 introduced `FallbackSubscription<P, F>` — a
   use-site-owned object whose `Watch` lives in the use-site node's
   `handles` vec and fires its own staleness callback. Once cross-file
   dependencies are mediated entirely by registry watches, the
   `dynamic_links` field on `NodeData` and the associated `RegistryQuery` /
   `LinkMode` machinery from 0006/0008 became dead surface.

2. **The pull-recompute state machine has no consumers.** `CacheState`,
   `Generation`, `Graph::mark_stale`, `Graph::mark_fresh`, and the
   `Computing` cycle-detection variant ship in `beans-core::graph` today.
   Nothing reads them. The Tier-3 tests in `tests/lifecycle.rs` are all
   `#[ignore]`d with the comment "no `Graph::pull` function exists."
   Whatever shape lazy recomputation eventually takes for diagnostics will
   likely differ from CST recomputation and from view-node recomputation;
   baking one shape (`recompute(&mut self, ...)`) into a graph-level trait
   pre-judges that.

The lesson the registry refactor taught us applies equally to the graph:
**name concrete patterns at their use sites; do not generalize for
hypothetical consumers.** `MultiQuery<N>` was rejected for
`FallbackSubscription<P, F>` for the same reason. The
`CacheState`/`Generation`/`stable` machinery is the graph-level analogue —
overshoot scaffolding that costs API surface and conceptual budget without
serving a real consumer.

## Decision

The layer-1 graph is **a typed arena with a hard-link forest and RAII
handles**, and nothing more.

```rust
struct NodeData<P> {
    payload: P,
    parent: Option<NodeId>,
    children: Vec<NodeId>,              // hard links
    handles: Vec<Box<dyn NodeHandle>>,  // RAII anchors
}
```

Removed from layer 1:

- `CacheState` enum and `Generation` counter.
- `NodeData::state` field.
- `Graph::mark_stale`, `Graph::mark_fresh`, `Graph::current_generation`.
- `NodeBehavior::on_destroyed` (RAII handles do all cleanup).
- The `dynamic_links: Vec<DynamicLink>` field on `NodeData` specified by
  ADR-0006 (never built as a graph field; replaced in practice by registry
  watches in `handles`).
- The graph-level stable-vs-volatile node distinction.

Retained at layer 1:

- The arena, generational `NodeId`, hard-link parent/children edges, and
  RAII `handles` vec.
- `NodeBehavior::on_created` for installing handles after a node enters
  the arena.
- `Drop` as the GC mechanic: destroying a node walks its hard-link
  subtree, frees descendants, bumps generations; each freed
  `NodeData::handles` drops, removing registry entries automatically.

Lazy recomputation, push-stale propagation, and stable-vs-volatile lifecycle
behavior become **layer-2 consumer concerns**. The pattern is expected to
look like a `Computed<V>`-shaped consumer-owned type that holds its own
subscriptions, caches a value, and recomputes on demand — but the concrete
shape is deferred until a real layer-2 consumer (diagnostics is the obvious
first one) drives the design.

The architecture is now three layers:

1. **Data layer** — `Graph<P>` and `Registries`. Storage and indexing.
2. **Analysis layer** — diagnostics, type resolution, dependency analysis.
   Builds on layer 1; not yet implemented.
3. **LSP layer** — `beans-lsp`. Builds on layers 1 and 2.

## Consequences

**Positive.**

- Layer 1 fits in one head. `NodeData` is four fields; `Graph` is an arena
  with `insert`/`get`/`destroy`/`iter`. The module is a few hundred lines.
- No speculative scaffolding to maintain. Removing `CacheState` et al.
  deletes ~200 lines of code, four `#[ignore]`d tests, and several
  module-doc paragraphs describing features that did not exist.
- Each lazy-recompute consumer can pick the shape that fits its dependency
  graph. Diagnostics may want a per-file cache keyed on path; completion may
  want a per-cursor cache; CST may want a different model. None of these
  need to share a `recompute` trait method.
- The cycle-detection question collapses: hard-link traversal is acyclic by
  construction, registry resolution is O(1), and any layer-2 cycle between
  consumer-owned `Computed<V>` values is caught for free by `RefCell`
  re-entrancy panic.
- Registry watches and graph RAII compose naturally: a layer-2 `Computed<V>`
  holds a `Subscription` or `FallbackSubscription` whose watch fires the
  consumer's invalidation callback. No graph-level participation required.

**Negative.**

- The lazy-invalidation contract that lived in 0009/0010 is no longer
  asserted by any code or test. We must remember to encode it when layer 2
  lands; this ADR is the durable pointer to that contract.
- A reader looking for "how does invalidation work?" is now pointed at "the
  layer-2 consumer that owns the subscription" rather than at a single
  graph-level explanation. The narrative is split across consumers.
- Pre-warming and coordinated scheduling become per-consumer concerns
  rather than graph-level. Any warmer must enumerate the cohorts of
  consumer-owned `Computed<V>` instances and call into each consumer's API;
  there is no shared `Graph::pull` entry point to dump speculative work
  into. In practice this is mostly the right place for warming policy
  (which is inherently per-type), but a future cross-type scheduler must do
  its own bookkeeping rather than ride free on the graph's iteration shape.

**Neutral.**

- ADR-0011's concern (LSP subscription handles surviving
  `Ctrl+A, Backspace`) still holds; the answer is mechanically different.
  The LSP holds a `Watch` from a `FallbackSubscription` whose registry
  survives volatile churn. The watch is the durable handle; no graph-level
  "stable node" category is needed to provide one.
- **Snapshot / fast-restart support is artifact-shaped, not engine-shaped.**
  Layer-1 (arena + registries) snapshots byte-stably; layer-2 *engines* are
  runtime-only. User-visible artifacts (diagnostics, document symbols,
  inlay hints) cache as serializable values alongside the graph snapshot,
  displayed immediately on restart and reconciled lazily as the engine
  recomputes (see [ADR-0028](0028-stale-while-revalidate-posture.md)). This
  means layer-2 consumer types do not need to be `Serialize`; the
  constraint lives at the artifact boundary, not inside the engine.
  Subscription callbacks will need a serializable enum shape
  (`MarkStale(NodeId)` for the common case) rather than `Box<dyn Fn>` —
  unblocked but load-bearing for fast-restart, deferred to its own ADR.

## Alternatives considered

**Keep the layer-1 lazy-recompute machinery as baseline.** Leave
`CacheState`, `Generation`, `mark_stale`/`mark_fresh` in the graph; layer-2
consumers use or ignore them. Rejected because (a) no current consumer
reads any of it, and (b) committing to a single recompute shape at layer 1
pre-judges every layer-2 design before its first use case lands. The same
`MultiQuery<N>` rejection logic from ADR-0008 rev 3 applies.

**Add a layer-2 `Computed<V>` skeleton in the same PR.** Land a stub type
for "subscribe + cache + recompute" so future consumers have a target.
Rejected because the shape will likely be wrong without a concrete driver:
diagnostics, CST, and view nodes all have different dependency profiles,
and we don't yet know whether one type covers all three. Wait for the first
real driver to anchor the design.

**Keep stable-vs-volatile as a graph-level flag and drop everything else.**
Rationale: even without lazy recompute, LSP subscription handles need to
survive content-clearing edits, and a `stable: bool` flag on `NodeData`
would document the intent. Rejected because the LSP's subscription is a
`Watch` whose lifetime is tied to its registry, not to a graph node. The
registry survives volatile churn already; the flag would be inert.

**Write `Computed<V>` in a separate crate (e.g., `beans-analysis`).**
Stronger crate boundary forces the layering question. Rejected because
ADR-0019 commits us to a single-core-crate design; the layer-2 type can
live as `beans-core::analysis` when it materializes. Splitting now is
premature.
