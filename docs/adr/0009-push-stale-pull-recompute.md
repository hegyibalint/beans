# ADR-0009: Use push-stale plus pull-recompute for invalidation

## Status

Superseded by [ADR-0027](0027-slim-graph-defer-recomputation-to-layer-2.md).

## Context

When a file changes, the parts of the semantic graph that depended on
that file are no longer valid. Diagnostics for files that referenced
its symbols must be reconsidered. Completion candidates that included
its members must be re-derived. Every cached value rooted in the
changed content is suspect.

But "the parts that depended on the change" is a transitive set that
can include many files. In a project with cross-language references,
a single edit in `Service.kt` might invalidate caches in dozens of
Java files that imported the projected JVM symbol. Recomputing all of
them on every keystroke is wasteful — most are not visible, not
focused, and the user will never look at their diagnostics until they
open them.

Conversely, doing nothing on change leaves stale values cached, and
the next pull (from the LSP, on user request) returns wrong answers.
We need an invalidation strategy that catches every dependent on
every change but pays minimal cost up-front and only does real work
when a value is actually requested.

## Decision

Two-phase invalidation:

- **Push-stale (eager, cheap).** When a file changes, tree-sitter
  diffs identify the affected CST nodes. Registries that those nodes
  registered in fire callbacks to all subscribers, marking them
  `Stale`. Marking is just flipping a bit (or bumping a generation
  counter); no value is computed. Staleness propagates upward through
  dynamic links: if `App.java`'s reference-to-`Service.process` is
  stale, `App.java`'s diagnostics are stale, `App.java`'s document
  symbols are stale, and so on. The push phase touches O(dependents)
  nodes and does no actual work on any of them.

- **Pull-recompute (lazy, on demand).** When the LSP queries a value
  — diagnostics for the visible file, hover for a token, completion
  at a cursor — the graph walks down from the requested node. Fresh
  nodes return their cached value immediately. Stale nodes recompute
  themselves, recursively pulling their dependencies (which may
  themselves be stale and recompute). The pull phase touches only
  the actual ancestor chain of the requested node, computing only
  what's needed for that specific answer.

Closed files, files the user isn't looking at, generated diagnostics
no one will read — all of these stay stale forever, and that's fine.
Their staleness is a marker that says "if you ever want this value,
recompute it"; if no one ever wants it, no work is done.

## Consequences

**Positive.**

- Edits are cheap. Typing in a hot file marks dependents stale in
  microseconds. The user sees no latency from invalidation itself.
- Pulls are bounded. Diagnostics for the visible file recompute only
  what they need; everything else stays untouched.
- Memory is bounded too. We don't accumulate work queues of "things
  to eventually recompute." Stale just means stale.
- The model maps cleanly to the LSP's request-driven nature. The
  protocol is built around "client asks, server answers" — push-
  stale-pull-recompute mirrors that exactly.
- Closed-file diagnostics are an explicit non-goal: if you don't open
  a file, we don't waste cycles validating it. When you open it, we
  catch up.

**Negative.**

- The first pull after an edit can be slow if a deep chain is stale.
  In practice this is rare — most edits invalidate small subgraphs —
  but a global change (e.g., adding a new dependency JAR) can
  invalidate broadly, and the next request pays the cost of
  recomputing whatever the user just asked for.
- Push-stale with subscription propagation is non-trivial to
  implement correctly. Each registry must reliably fan out to all
  subscribers. Bugs here manifest as "stale value never invalidated"
  — silent incorrectness, hard to debug.
- The invariant "Stale means you must recompute before reading" must
  be enforced everywhere. A code path that reads a node's cached
  value without checking state is a wrong-answer bug. We mitigate
  this by gating cached reads through a single accessor that handles
  the state check, but discipline is required.

**Neutral.**

- We have no strong "freshness everywhere, always" guarantee. If the
  user asks "show me errors in every file in the project right now,"
  we may have to recompute much of the graph. This is the right
  tradeoff for an interactive tool, but worth knowing about for
  batch-mode use cases (e.g., CI integration), which would warrant a
  different driver.

## Alternatives considered

**Synchronous direct recomputation on change.** Every keystroke,
walk all dependents and recompute their values immediately. Rejected
for obvious reasons: editing latency tied to dependency size, and
99% of recomputed values are never read.

**Push-update-eagerly.** When a file changes, push new values
through the dependency graph instead of marking stale. Same shape as
above but propagates values rather than recomputing on demand.
Rejected because it's still eager and wasteful — pushing values
through to closed-file diagnostics that no one reads. The benefit
("everything is always up to date") doesn't outweigh the cost in an
LSP context.

**Lazy invalidation only — no push.** Don't mark anything stale on
change. Instead, on every pull, traverse the dependency graph and
check timestamps to decide what to recompute. Rejected because it
flips the cost: cheap edits but expensive reads. Reads happen during
user interactions where latency is most visible. Push-stale's O(1)-
per-node cost on edits is much cheaper than per-read traversal.

**Generation-based invalidation without subscriptions.** Bump a
global generation counter on each edit; nodes compare their
generation to the current one on read. Rejected because it conflates
"changed" with "every node is stale" — every read after every edit
walks the dependency tree. We want fine-grained staleness so that
unrelated reads stay fast. This means subscriptions; this means
push-stale.
