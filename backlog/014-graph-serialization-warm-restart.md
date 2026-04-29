---
status: pending
area: graph
priority: medium
---

# Serialize the graph for warm restart

## Description

Persist the stable portion of the semantic graph (JMOD nodes, parsed
dependency-jar nodes) to disk so subsequent startups can warm-load the
state instead of re-deriving it. On startup, deserialize the snapshot,
mark stable nodes as fresh, and run a background diff to detect and
recompute any nodes whose inputs have changed since the snapshot was
taken.

Scope:

- Define a binary format for `CacheNode` payloads and links. Versioned.
- Distinguish stable from volatile nodes per ADR-0011 — only stable
  nodes are serialized.
- Snapshot writer: walk the stable subgraph, write to disk.
- Snapshot reader: deserialize and integrate into a fresh engine.
- Background diff: compare snapshot inputs (JDK version, dependency jar
  hashes, module-info contents) against the live filesystem; mark
  affected nodes stale.

## Context

Cold startup of a multi-thousand-class JDK plus dependencies is expensive.
Warm restart trades a one-time serialization cost on shutdown (or
post-indexing) for instant startup on every subsequent session.

ADR-0009 (push-stale + pull-recompute) makes this clean: deserialized
nodes start fresh, and the background diff stale-marks any that need
recomputation. Pull-recompute then handles the rest lazily.

ADR-0011 explicitly distinguishes stable nodes; volatile nodes (open
buffers, in-flight edits) are never serialized.

## Acceptance criteria

- A snapshot can be written and read back with byte-identical results
  for the same inputs.
- Warm startup is measurably faster than cold startup (target: under 500ms
  for a typical workspace).
- Changing a dependency jar between sessions causes the affected nodes
  to be stale-marked correctly on next startup.
- The format is versioned; an out-of-version snapshot is discarded
  cleanly without crashing.
