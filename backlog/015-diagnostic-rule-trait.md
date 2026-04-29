---
status: pending
area: graph
priority: medium
---

# Define and adopt the diagnostic rule trait

## Description

Define a `Rule` trait whose implementations produce diagnostics from a
context that exposes registry queries and the affected node payload.
Rules register with the engine; the engine invokes them as part of the
pull-recompute pipeline so diagnostics participate in the same caching
and invalidation as everything else.

Sketch:

```rust
trait Rule {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic>;
}
```

Each rule should be small and single-purpose (one check per rule), so
adding or disabling a rule is a one-line change.

Initial rules to land alongside the trait:

- "Reference not found"
- "Inaccessible member"
- "Sealed switch not exhaustive" (depends on backlog 008)
- "Unused import"

## Context

Rules per ADR-0017 live in their language modules and run as part of the
node enrich step. There is no central rule pipeline; the engine merely
provides the trait and the registration hook.

Diagnostic recomputation should reuse all the cache-state machinery from
ADR-0009 — diagnostics are just another node payload, so they go stale
when their inputs change and recompute on pull.

## Acceptance criteria

- The `Rule` trait exists in `beans-core` with documented `RuleContext`.
- At least four rules are registered and produce diagnostics through the
  graph.
- Diagnostic recomputation only runs for rules whose inputs went stale.
- Fixture tests assert that a syntactically valid but semantically
  broken Java file produces the expected diagnostics.
