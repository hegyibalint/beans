# ADR-0007: Use NodeId as runtime-only identity; semantic identity lives in registry keys

## Status

Accepted

## Context

The graph holds many nodes — files, CSTs, language symbols, JVM
projections, view nodes — and we need a way to refer to them. Two needs
collide:

1. **Internal traversal must be fast.** The graph engine walks
   parent→child links, looks up cached values, marks staleness, and
   resolves dynamic-link queries millions of times per editing session.
   Whatever identity we use must support O(1) arena indexing.

2. **External APIs must be stable.** The LSP server, language rule
   authors, and the snapshot format on disk all need to refer to "the
   same method" across file edits, parser re-runs, restarts, and
   cross-session lookups. A method might disappear and reappear; a use
   site might reconnect to a different definition in a different
   language. The reference must be meaningful in those terms — names,
   FQNs, kinds — not in terms of arena slot numbers.

If we conflate these, we either give the LSP arena indices (which become
stale after any restart and have no semantic meaning) or we make every
internal hot-path traversal go through hash lookup on string-based
identity (slow, allocating).

## Decision

Two distinct identity systems, neither pretending to be the other.

- **`NodeId` (a `u64`) is the runtime arena index.** It is fast,
  copyable, comparable. It is preserved verbatim when the entire graph
  is serialized to a snapshot and reloaded — because in that case the
  whole arena round-trips, so the indices remain consistent. It is
  **not** stable across rebuilds from source, across version upgrades
  that change node layout, or across any other operation that doesn't
  preserve the full arena. Internal graph code uses `NodeId`
  exclusively.

- **Semantic identity is the registry key.** The thing that uniquely
  identifies "the method `process(String)` on class
  `com.example.Service`" is the rich query object you'd use to find it
  in the registry: language tag, FQN, signature, kind. Looking up a
  semantic identity always goes through a registry, which returns a
  current `NodeId` (or none). External APIs — LSP messages, rule code,
  the snapshot's cross-references between file groups — speak in
  registry keys, never in `NodeId`s.

A registry key looks up to a `NodeId` for any given moment in time, but
the same key can resolve to different `NodeId`s across rebuilds, and
that's fine — the key is the identity, not the slot.

## Consequences

**Positive.**

- Hot-path graph traversal is just integer indexing into a `Vec`. No
  string hashing, no allocation.
- LSP-facing handles are durable in the only sense that matters:
  semantic. A user's "go to definition" target stays meaningful across
  restarts because the LSP refers to it by FQN+signature, and that key
  re-resolves on the new graph.
- Rule authors don't need to think about identity churn. They never
  see a `NodeId`; they call `ctx.symbol("com.example.Service.process")`
  and get the current node, whatever its slot.
- Snapshot format is simpler: serialize the arena verbatim (NodeIds are
  internally consistent), serialize the registries, done. No identity
  rewriting needed.

**Negative.**

- Two-step resolution on the boundary: semantic key → NodeId → value.
  This is one extra hash lookup compared to a direct pointer. Cost is
  paid only on external API entry, not on internal traversal, so the
  amortized cost is small.
- Contributors must internalize the rule: "if it's leaving the graph
  module, convert NodeId to a registry key first." This is a code-
  review-and-API-shape concern more than a runtime one.
- A `NodeId` that escapes to external code (e.g., logged, embedded in a
  message that survives a restart) becomes a bug. We mitigate this by
  not exposing `NodeId` in any public type signature — internal only.

**Neutral but worth flagging.**

- The snapshot reload path preserves `NodeId` because the whole arena
  is preserved. If we ever needed to merge two snapshots, or replay a
  snapshot onto a different code version, we'd have to do an
  arena-level rewrite. That's expected and acceptable.

## Alternatives considered

**Use semantic addresses (URIs) as primary identity throughout.** Drop
`NodeId` entirely and use strings like `kt://com.example.Service.process`
as the universal handle. Conceptually clean, very stable. Rejected
because every internal traversal would pay string hashing cost, every
node would carry a heap-allocated string, and parent→child traversal
becomes a hash lookup instead of a vector index. The graph performs
traversal far more often than it crosses the API boundary — optimizing
for the rare case at the expense of the common case is the wrong
tradeoff.

**Use stable opaque IDs (UUIDs) at the graph level.** Generate a
session-stable UUID per node and use that everywhere. Rejected because
UUIDs are 16 bytes, not 8, and they don't index a `Vec` directly — you
still need a UUID→slot map, which is a hash lookup. You lose the speed
of NodeId without gaining the semantic meaning of a registry key.

**Hybrid: NodeId internally, semantic URI in cached fields, no
registry.** Each node carries its own URI alongside its `NodeId`. The
URI is the public face. Rejected because lookup of "give me the node
for this URI" still needs an index — i.e., a registry. We'd rebuild
the registry organically and call it something else. Better to keep
the registry explicit and central.
