---
status: pending
area: graph
priority: medium
---

# Snapshot and fast-restart strategy

## Description

Persist a per-workspace snapshot to disk so subsequent startups can warm-
load layer-1 state plus the most recently displayed user-visible artifacts,
satisfying the stale-while-revalidate posture
([ADR-0028](../docs/adr/0028-stale-while-revalidate-posture.md)).

Three things go in the snapshot:

1. **Layer-1 graph + registries.** Byte-stable serialisation of the arena
   (`Vec<Slot<P>>`, free list, generation counters) plus registry state
   (`providers: HashMap<K, Vec<NodeId>>` and the subscriber lists). NodeIds
   round-trip verbatim because the arena is preserved byte-for-byte. Per
   ADR-0007 this tightens the "NodeId is not stable across snapshot
   reload" caveat to "NodeId is stable across snapshot reload *iff* the
   snapshot preserves the arena byte-for-byte."

2. **Layer-2 artifact cache.** A small bag of serializable values that
   the LSP last published to the client: diagnostics, document symbols,
   inlay hints — keyed by file path. Tens to hundreds of entries, not
   millions. These are *outputs*, not engine state; the layer-2 engine
   itself is runtime-only and rebuilds cold each session
   (per ADR-0027).

3. **External-resource fingerprints.** mtime (or content hash) per
   `dependency://` and `jmod://` resource so the loader can detect
   what changed between sessions.

On startup:

1. Read snapshot. If the format version mismatches or the file is
   corrupt, discard cleanly and fall back to cold start.
2. Deserialize the arena and registries. NodeIds in the registry's
   provider lists and in any cached artifact references stay valid
   because the arena round-trips byte-for-byte.
3. Replay the artifact cache: emit `publishDiagnostics`,
   `documentSymbol`, `inlayHint` for each cached entry. The user sees
   their last-session squiggles within the snapshot-load budget.
4. Validate external resources: per `dependency://` / `jmod://` mtime
   check. Mismatched resources mark their dependents for reconciliation
   and may trigger destroy + re-integrate of affected subtrees.
5. Start the layer-2 engine; reconcile changed files lazily (or via a
   low-priority background sweep) per the stale-while-revalidate ADR.
   `window/workDoneProgress` is the user-visible signal.

## Constraints this introduces on layer 1

- **Subscription callbacks must be a serializable enum**, not
  `Box<dyn Fn>`. The 99% case is `InvalidationCallback::MarkStale(NodeId)`
  (or whatever shape layer 2 ends up using). An escape-hatch
  `Custom(Box<dyn Fn>)` variant is allowed but is not snapshotted —
  custom-callback subscribers re-register on load via their owner's
  `on_created`. Until snapshot work begins, the registry can keep its
  current arbitrary-closure shape; this constraint kicks in when this
  item starts.

- **Payloads must implement `Serialize` / `Deserialize`** (or whatever
  the chosen format requires). Today's payloads (`JvmNodePayload`,
  `JavaNodePayload`, `NodePayload`) hold owned data and `NodeId`
  references; the serialization should be straightforward. The choice
  between bincode (simple, copying), rkyv (zero-copy, more constrained),
  or postcard (small footprint) is part of this item.

## Acceptance criteria

- A snapshot can be written and read back; the loaded graph + registries
  pass the same invariants as a freshly built one (registry-cleanup,
  hard-link cascade, generation-aware `get`).
- Re-displaying cached artifacts on warm startup hits the under-a-second
  target on a project of ~1M nodes (rough budget per ADR-0028 design
  notes: ~300-400 ms with serialized registry state, ~750-1500 ms if
  registries are reconnected via `on_created` re-walk).
- Changed `dependency://` / `jmod://` resources between sessions cause
  the affected subtrees to be destroyed and rebuilt without crashing or
  stranding stale subscriber entries.
- The format is versioned; an out-of-version snapshot is discarded
  cleanly without crashing the LSP.
- Code actions are suppressed on cached-but-not-yet-reconciled
  diagnostics until the corresponding file's engine has caught up
  (per ADR-0028).

## Open questions

- **Format choice.** rkyv (zero-copy) wins on load latency but constrains
  payload representation; bincode is unconstrained but pays a copy. The
  load budget makes rkyv tempting, but profiling against real
  payload sizes is the right way to decide.
- **Reconnect-via-`on_created` vs serialize-registry-state.** The latter
  is faster (skips the per-node walk) but requires the serializable-
  callback constraint. The former is slower but keeps the registry
  callback shape unchanged. Pick when implementing.
- **What gets reconciled when.** All cached files at startup, or
  per-file on first user attention? The latter saves work but feels
  like more bookkeeping. Probably resolve via measurement.
