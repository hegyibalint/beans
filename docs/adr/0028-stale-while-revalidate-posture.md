# ADR-0028: Stale-while-revalidate is the default posture

## Status

Accepted.

## Context

JVM-language LSPs are widely characterized by long blocking startup —
"indexing project…" for minutes — followed by a brief useful window before
the next reload. Eclipse JDT, Red Hat's Java extension, and (in cold-cache
states) IntelliJ all ship some version of this experience. Users routinely
call out the dead time as the worst part of working with these tools.

The standard implementation choice that produces this failure mode is
*correctness-first*: hold off on serving any results until the index is
complete, on the theory that wrong information is worse than no
information. The result is a multi-minute window where the editor is
visibly running but functionally inert.

Beans positions itself against that. The graph + registry layers (per
[ADR-0027](0027-slim-graph-defer-recomputation-to-layer-2.md)) restore
quickly from snapshot; the layer-2 engines that produce user-visible
artifacts (diagnostics, document symbols, inlay hints) are runtime-only
and must rebuild on each session. Without an explicit posture, the natural
implementation is to block until those engines have caught up — which
recreates the problem we are trying to avoid.

## Decision

Beans serves last-known artifacts immediately on startup (and on any
analogous "would otherwise block the user" event), and revalidates them in
the background. The user is never gated on the engine catching up.

- **Proactive artifacts** (squiggles, document symbols, inlay hints) are
  cached as serializable values across sessions and replayed on load.
- **Reactive features** (hover, go-to-def, completion, find references)
  run cold against the freshly loaded graph; their latency budget is
  per-request (~100 ms), not whole-session.
- **Revalidation progress is exposed** via `window/workDoneProgress` so
  the user sees a status indicator while reconciliation runs. The
  indicator is honest: "we are showing you cached results; fresh ones are
  computing."
- **Code actions and quick fixes are suppressed** on cached-but-not-yet-
  reconciled diagnostics. A fix that applies to a problem that may no
  longer exist is dangerous; the squiggle is still useful as a *display*
  of last-known state without offering action on it.
- **Reconciliation is per-file and lazy.** As each file's engine produces
  fresh artifacts, the cached entry is overwritten and a fresh
  `publishDiagnostics` is sent to the client. Files that no longer exist
  on disk drop their cache entries; files unchanged since snapshot keep
  theirs without recomputation.

The principle generalizes beyond startup. Any time computation would block
the user-perceptible interaction loop, the answer is "show last-known and
revalidate," not "block." File-change reconciliation, dependency-graph
updates, project-import refreshes — all fit the same pattern.

## Consequences

**Positive.**

- Sub-second time-to-useful even on large projects. Beans is differentiated
  against the existing JVM-LSP failure mode the moment it ships.
- The user is never blocked on indexing. Editing, navigation, and most
  reads work continuously across the session, including the boundary at
  startup.
- Implementation freedom at the engine layer. Layer-2 consumers do not
  need to be designed for incremental availability — they just compute
  when asked. The "make it appear available" responsibility lives at the
  artifact-cache + reconciliation layer, separate from the engine.
- Honest UX. The `window/workDoneProgress` indicator tells the user
  exactly what state the editor is in; the user does not have to guess
  whether absence of squiggles means "no problems" or "not yet checked."

**Negative.**

- Brief staleness window (milliseconds-to-seconds) where displayed
  artifacts may be wrong. A squiggle for a problem the user already fixed
  remains visible until reconciliation completes; conversely, a real
  problem in a freshly-edited file may not be flagged until the engine
  catches up.
- Code actions briefly unavailable for stale entries. If the user wants to
  apply a quick fix on a cached squiggle, they wait until the file is
  reconciled. This is a deliberate safety choice, but it produces a small
  latency hit on actions that target just-restored state.
- Reconciliation requires a per-engine catch-up mechanism: a low-priority
  background scheduler that sweeps changed files, plus per-file dirty
  tracking to skip work the snapshot already covers.
- A persistent disagreement between cache and engine (engine bug producing
  different results than the cache) shows up as squiggles that flip on
  reconciliation. Diagnosable in the workDoneProgress logs but worth
  watching for in QA.

**Neutral.**

- The posture is implementation-agnostic. The snapshot format, the cache
  format, and the catch-up scheduler are all deferred to their own ADRs;
  this ADR commits only to the user-facing behavior.

## Alternatives considered

**Block until ready.** What most existing JVM LSPs do. Rejected as the
failure mode we are explicitly trying to avoid. The user-experience cost
is obvious (multi-minute dead time) and is the single most-cited weakness
of JVM tooling.

**Show nothing until ready, no spinner.** Less honest than the
"revalidating" indicator. Users assume the absence of squiggles means "no
problems," which can mask real issues during the catch-up window. Rejected
because the safety property of the loading indicator is what makes the
stale-display acceptable.

**Show stale and run code actions on stale data.** Performance is the same
as the chosen approach but safety degrades — users can apply fixes that
break their files because the underlying problem no longer exists.
Rejected; the "no actions on unreconciled entries" rule is the cost of
stale-display being safe.

**Two-tier startup: block briefly, then become responsive.** E.g., block
for the first 1-3 seconds while the graph + registries load, then unblock
once the cached artifacts are ready. Rejected because the unblock latency
is bounded by snapshot-load time, which scales with project size; on the
largest projects, "briefly" stretches into seconds. The simpler invariant
("never block") is more robust and produces a more predictable UX across
project sizes.

**Eager full revalidation in the background.** Display cached artifacts,
then immediately recompute every file's diagnostics in parallel, replacing
the cache wholesale. Rejected because most files don't actually change
between sessions; eager full revalidation pays compute cost for files that
the cache already correctly represents. The lazy per-file scheme catches
up at the speed the user actually navigates, which matches their attention.
