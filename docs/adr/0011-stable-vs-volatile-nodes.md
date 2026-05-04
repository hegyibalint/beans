# ADR-0011: Distinguish stable nodes from volatile nodes

## Status

Superseded by [ADR-0027](0027-slim-graph-defer-recomputation-to-layer-2.md).

## Context

The graph contains nodes with very different lifecycles. A
`file://path/to/Service.kt` node represents a path on disk; the path
is stable, but the file's *content* changes constantly. A
`cst://Service.kt` node represents the parsed tree of that content;
the tree is rebuilt whenever the content changes. Inside the CST
sit language symbols (`kt://com.example.Service`) and JVM projections
(`jvm://com.example.Service`); both depend on the parse and are
rebuilt with it.

Above the language layer, the LSP-facing nodes — diagnostics,
document symbols, inlay hints, code lenses — also have a path.
`diagnostic://Service.kt` is "the diagnostics for the file at this
path," not "the diagnostics for this particular tree the parser
produced." When the user runs `Ctrl+A, Backspace`, the file content
clears and re-parses; the CST and language symbols are destroyed and
re-created with new `NodeId`s. But the LSP client still has a live
subscription handle for diagnostics on `Service.kt`. If the
diagnostic node also got a new `NodeId`, the subscription would
break — every edit cycle would force the client to re-subscribe.

We need a clear policy for which nodes are durable across content
changes and which are not.

## Decision

Two categories of nodes:

- **Stable nodes** have identity tied to an external resource that
  outlives any particular content snapshot. Their `NodeId` is
  preserved across content changes. Examples:
  - `file://path/Service.kt` — the path exists whether or not the
    file is open, populated, or empty.
  - `dependency://maven/group:artifact:version` — a dependency
    coordinate from the build system.
  - `jmod://java.base` — a JDK module.
  - LSP view nodes parented to a file: `diagnostic://Service.kt`,
    `document_symbols://Service.kt`, `inlay_hints://Service.kt`.
    These are stable because the LSP client's subscription handles
    must survive editing cycles.

- **Volatile nodes** are derived from content and are recreated when
  the content changes. Examples:
  - `cst://Service.kt` — parse trees are not preserved verbatim
    across re-parses, even if the new tree is semantically identical.
  - Language symbols (`kt://...`, `java://...`, etc.) — recreated
    with the parse.
  - JVM projection nodes — created when language nodes appear,
    destroyed when they go.

When a stable node's value updates (e.g., `file://Service.kt`'s
content changes), the node persists; only its cached `value` field
is replaced and its dependents marked stale. When a volatile node's
underlying source disappears, the node itself is destroyed via the
hard-link GC walk (see ADR-0006).

LSP view nodes are deliberately classified as stable, parented under
their file node by hard link. The file node's identity is stable, so
the view nodes hard-linked under it are stable too: the GC walks from
the file downward, but the file itself isn't destroyed by content
changes — only its volatile children (the CST and what depends on
it) are.

## Consequences

**Positive.**

- LSP subscription handles survive `Ctrl+A`, `Backspace`, `Ctrl+Z`,
  and other content-clearing edit sequences. The client's
  registration to `diagnostic://Service.kt` is stable across the
  full editing lifecycle of the file, including transient empty
  states.
- The delete/restore cycle for file *content* is well-defined:
  empty content destroys volatile children, file node persists,
  view nodes persist, dynamic-link subscribers (in other files)
  retain their subscription on the now-missing definitions.
- External-resource nodes (`dependency://`, `jmod://`) compose
  naturally with the rest of the graph. A dependency JAR loading
  asynchronously is just an updated value on a stable node.
- The model maps cleanly to user mental model: "the diagnostics for
  this file" is a thing, not a fresh-on-every-keystroke ephemeron.

**Negative.**

- Two kinds of nodes means two cleanup paths. Volatile nodes
  destroyed via hard-link GC; stable nodes never destroyed by GC
  (only by explicit user action, e.g., closing a workspace). The
  rules around when a stable node *can* be destroyed must be
  clear and enforced.
- Stable nodes hold memory even when the file is closed and
  uninteresting. We mitigate this with a separate eviction policy
  for stable nodes (LRU on closed files, e.g.), but the policy
  is independent of the GC walk and adds implementation surface.
- Authors of new node types must decide: stable or volatile? The
  default should be volatile (less special, simpler lifecycle).
  Stable is reserved for nodes that genuinely represent durable
  external resources or LSP-facing handles.

**Neutral.**

- Stable nodes hard-linking volatile children is a normal
  arrangement, but it means "destroy this file's CST" is a
  per-volatile-child operation, not a destroy-the-parent cascade.
  The GC walk descends from a destroyed root (a volatile root); a
  stable node above the volatile root is fine.

## Alternatives considered

**Destroy `file://` on every content change.** Treat the file node
as volatile too — destroyed when content changes, recreated with a
new `NodeId`. Simpler model, only one node category. Rejected
because it forces every dependent (LSP subscriptions, dependent
files' dynamic links to it) to re-resolve on every edit. The LSP
client would either need to resubscribe constantly, or we'd need a
separate stability layer above the graph. Adding that layer is
exactly the stable/volatile distinction we're already making — just
in a less obvious place.

**Make everything volatile.** All nodes recreated on content
changes; identity comes purely from registry keys (per ADR-0007).
Conceptually pure: NodeId is just a slot, semantic identity is the
key, why bother preserving slots? Rejected because the LSP client's
subscription must point to *something* on our side. If everything
is volatile, the LSP integration layer needs to maintain its own
mapping from "long-lived subscription handle" to "current NodeId,"
which is a separate stability layer doing exactly what stable nodes
do. We may as well centralize it.

**Make everything stable.** Never destroy nodes; just zero out
values when source is gone, recreate on demand. Avoids GC entirely.
Rejected because the volatile subgraph is *large* (a full project's
CSTs and language symbols can be hundreds of thousands of nodes per
parse), and we genuinely want to free that memory when the source
is gone. Stable-everywhere bloats the heap with dead nodes
indefinitely.

**Stable nodes only at top-level resources, not for view nodes.**
Files and dependencies are stable; diagnostics and document symbols
are volatile, parented to volatile CST. Rejected because then the
LSP subscription problem is back: client has a handle to
`diagnostic://Service.kt`, content changes, CST is destroyed, view
node is destroyed, handle is dead. The whole point of distinguishing
stable from volatile is to give the LSP something durable to
subscribe to.
