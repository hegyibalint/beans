---
status: pending
area: core
priority: medium
---

# Implement tiered subscriptions for dynamic links

## Description

`DynamicLink` currently has `active_index: Option<usize>` and
`cached_result: Option<NodeId>`. That carries enough state for resolution
itself, but not enough for the tiered subscription contract from ADR-0008:

- A value-watch on the *active* query (notify when the current target's
  value changes).
- An existence-watch on every *higher-priority* query that currently
  misses (notify when a higher-priority provider appears so the link can
  promote).
- No watch at all on lower-priority queries (they are dormant; the link
  will only consult them if the active query falls through).

Implementing this needs a richer state shape than the current option
pair: at minimum, the link needs to know "I was previously resolved to
provider X via query i; now I want to be told if a higher-priority query
gains a provider." That's a state machine `Unresolved | Cached { active,
target } | AllMissed`, plus the wiring for the engine to consult and
update these subscriptions when registry events fire.

## Context

Deferred during the big-bang migration (see commits on
`feat/graph-migration`). Steps 4-6 of that migration do not depend on
tiered subscriptions — the fixture rebuilds the graph per test and the
LSP server's diagnostic pipeline at step 6 returns empty results,
exercising no live invalidation through dynamic links.

Becomes load-bearing when:
- Diagnostic rules fire on cross-file edits and need to re-resolve their
  dynamic links automatically.
- The LSP needs incremental recomputation as the user types.

ADR-0008 calls this out as a deliberate part of the design.

## Acceptance criteria

- `DynamicLink<Q>` carries enough state to express the three
  subscription positions (active value-watch, higher-priority existence-
  watch, dormant lower-priority).
- Registry register/unregister events flow through to the link's
  subscription state and update `(active_index, cached_result)` without
  manual `invalidate()` calls.
- Tests cover: active query gains/loses provider, higher-priority query
  appears (active demotes to lower), all queries miss then a provider
  appears.
- No regressions on the existing 7 dynamic-link mechanics tests.
