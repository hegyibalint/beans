# ADR-0006: Distinguish hard links and dynamic links in the graph

## Status

Accepted

## Context

The semantic graph wires together many node types: file nodes, CST nodes,
language symbol nodes, JVM projection nodes, and LSP-facing view nodes
(diagnostics, document symbols, inlay hints). Some of these relationships
are intra-file and deterministic — a file owns its CST, a CST owns its
parsed symbols, a Kotlin symbol owns its JVM projection. Other
relationships cross files and are inherently uncertain — a use site in
`App.java` references a method that might live in Java, in Kotlin (via JVM
projection), or might not exist at all. The latter must survive the file
they target being deleted and recreated, must support fallback resolution
across languages, and must respond to invalidation when a target changes.

If we treat both kinds of edges identically, we either pay registry/query
overhead on intra-file relationships that don't need it (slow, cache-
unfriendly), or we hardcode cross-file references and lose the ability to
re-resolve when files change (broken delete/restore cycles, no cross-
language fallback). The two relationships have fundamentally different
lifecycles and need different machinery.

## Decision

The graph has two edge types with different semantics and different
storage:

- **Hard links** are ownership/containment edges within a file's subtree.
  A file hard-links its CST; a CST hard-links its language symbols; a
  language symbol hard-links its JVM projection. Hard links are stored
  directly as `Vec<NodeId>` on the parent. When the parent is destroyed,
  its hard-linked children are destroyed (top-down GC walk). No registry
  is involved — these edges are private, deterministic, and never cross
  file boundaries.

- **Dynamic links** are cross-file dependency edges mediated by
  registries. A use site in one file referencing a definition in another
  file is a dynamic link. Dynamic links are not stored as direct target
  IDs; they are stored as a list of registry queries (with a cached
  result for the active query). Resolution happens through the registry,
  which means lookup, fallback across queries, and survival across the
  target file's deletion and restoration.

A node may have both: hard links to its children (always, for ownership)
and dynamic links to other files (if it has cross-file references).

## Consequences

**Positive.**

- Intra-file relationships are zero-overhead: a `Vec<NodeId>` lookup, no
  hash, no registry round-trip.
- Cross-file relationships go through one uniform mechanism — registries
  — which gives us subscription-based invalidation, delete/restore
  reconnection, and cross-language fallback (see ADR-0008) for free.
- GC is straightforward: walk the hard-link tree top-down from a deleted
  root; the tree is finite and acyclic by construction.
- The two edge types make rule authors' mental model clearer: "is this
  inside the same file or not?"

**Negative.**

- Two mechanisms instead of one. Contributors must remember which
  applies and there's a conceptual cost to learning both.
- Edge-case symbols that conceptually span files (e.g., a partial-class
  pattern, if we ever supported one) need to be modeled carefully. We
  don't currently have any such case in JVM languages, but the rule is
  not negotiable: hard links never cross files.
- Refactoring a use-and-def from one file into the same file (or vice
  versa) means changing edge type, not just rewiring a pointer. In
  practice this is rare in user code; it matters for our own model
  changes.

## Alternatives considered

**Single edge type with everything dynamic.** Treat every edge as a
registry-mediated lookup. Uniform model, simpler to explain. Rejected
because every CST→symbol, symbol→projection edge would pay a hash lookup
and registration cost — millions of edges in a real project, all
essentially deterministic. The performance hit is bad and the abstraction
is wrong: a file *contains* its CST, it does not *reference* it.

**Eager pointer references for everything.** Store target `NodeId` for
every edge, including cross-file. Uniformly cheap to traverse. Rejected
because it breaks delete/restore: if `App.java` stores a `NodeId`
pointing to `Service.process` and `Service.kt` is deleted then
re-created, the new `process` has a fresh `NodeId` and the old pointer
dangles. Reconnection would require scanning every node that referenced
the dead one, which is exactly what registries already do — so we'd just
be reinventing them.

**Two edge types but storing both in registries.** Keep the conceptual
distinction but route hard links through registries too, just for
consistency. Rejected as cargo-cult uniformity: the registry's purpose
is subscription and re-resolution. Hard links have neither — ownership
is not subscribed to, it's owned. Forcing them through a registry adds
cost without benefit.
