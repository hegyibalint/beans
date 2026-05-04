# ADR-0010: Recompute lazily on pull, never eagerly on stale-mark

## Status

Superseded by [ADR-0027](0027-slim-graph-defer-recomputation-to-layer-2.md).

## Context

ADR-0009 established that file changes mark dependent nodes stale via
a fast push phase. The natural follow-up question: *when* does the
recomputation actually run? There are two possibilities, and we need
to be explicit about which one we're committing to, because the
choice has wide-reaching consequences for performance, complexity,
and correctness.

Eager recomputation would say: as soon as a node is marked stale,
schedule its recomputation, either synchronously or on a background
worker. By the time anyone reads the value, it's either fresh or
in-flight.

Lazy recomputation would say: marking stale is just bookkeeping. The
value is invalid; nobody is going to use it yet. Recomputation only
runs when something actually pulls the value, and only for the
specific node being pulled (recursively pulling its dependencies as
needed).

The two strategies look similar but produce very different behavior
under load.

## Decision

**Recomputation is purely lazy and pull-driven.** Marking a node
stale does nothing beyond setting the state flag. No work is
scheduled, no thread is woken, no future value is materialized. The
only thing that triggers recomputation is a `pull(node_id)` from
something that actually wants the value — typically the LSP responding
to a client request, or another node recomputing itself and needing
its dependencies fresh.

When a pull hits a stale node:

1. Mark it `Computing` (cycle detection).
2. Recursively pull each of its dependencies, ensuring they're fresh.
3. Run the recompute function.
4. Mark `Fresh(generation)`, store the new value.
5. Return the value to the caller.

If multiple things are stale, they recompute in topological order
naturally — because each pull recursively pulls its dependencies
first, and the dependencies' values get cached on the way back up.

A node that no one ever pulls stays stale forever. That's a feature,
not a bug.

## Consequences

**Positive.**

- Wasted work is impossible by construction. We only compute values
  that are needed for an answer that's been asked for.
- Memory and CPU pressure during edits is minimal: even if a thousand
  nodes were marked stale, we don't pay anything until a pull comes.
- The LSP's request-response loop is the natural driver. The client
  asks for diagnostics; we recompute exactly what's needed for those
  diagnostics. No background pipeline competing for CPU with the
  user's typing.
- Behavior is deterministic and predictable. There are no
  "background recomputation finished too late" edge cases. The state
  machine is `Stale → Computing → Fresh`; nothing else.
- Closed-file values cost nothing. Stale equals "would be wrong if
  you read it"; if you don't read it, the cost is zero.

**Negative.**

- The first pull after an edit can have latency proportional to the
  staleness of the chain. If the user just opened a file that
  depends on many things that have been stale for a while (say, a
  dependency JAR was just loaded), the first hover or completion
  could feel slow. This is mitigated by the fact that staleness
  chains are usually shallow, but pathological cases exist.
- We can't pre-warm the cache. There is no notion of "compute this
  in the background while the user is idle." If we ever decide we
  want that, it's an additive feature: a separate driver that issues
  speculative pulls. The lazy core stays unchanged.
- Diagnostic latency on file-open can spike if a large amount of
  newly-relevant work is pending. The LSP smooths this with
  progress reporting, but it's a real cost.

**Neutral.**

- The `Computing` state must be handled correctly. If a recompute
  triggers a recursive pull that cycles back to itself, we detect
  the cycle (state is already `Computing`) and either return a
  partial value or report an error. This is correct, but the cycle
  detection logic must be robust.

## Alternatives considered

**Eager recomputation on stale-mark.** As soon as the push phase
marks a node stale, schedule its recompute. Two flavors:

- *Synchronous on the edit thread:* makes editing latency explicit
  and bad. A keystroke that invalidates a big subgraph hangs the
  editor. Rejected immediately.
- *Asynchronous on a worker thread:* better latency, but introduces
  background load that competes with the user's typing, and pays for
  recomputation of values that may never be read (closed files,
  unfocused diagnostics). The complexity (queue management,
  prioritization, cancellation when a node is re-staled mid-flight,
  consistency with new edits arriving) is substantial. Rejected.

**Hybrid: lazy by default, eager for specific high-priority node
types.** E.g., always eagerly recompute diagnostics for the visible
file. Tempting, but the LSP already pulls those nodes when the
client asks for diagnostics — eager recomputation just races the
client request, often losing. The pull-driven model already produces
the right behavior; an eager prefetch only helps if the user
consistently asks slightly *after* the edit settles, which they
usually do, but the savings are small. Rejected for now;
re-evaluate if profiling shows file-open latency is unacceptable.

**Eager recomputation but with cancellation.** Recompute eagerly,
but if a node is re-staled before its recompute finishes, cancel
the in-flight work. Avoids paying for outdated values, but
introduces all the complexity of in-flight cancellation, partial
results, and worker coordination. Rejected as a strict regression
versus pure lazy: more complexity, no clear benefit when reads are
already fast under lazy.
