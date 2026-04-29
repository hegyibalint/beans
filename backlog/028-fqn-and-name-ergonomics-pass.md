---
status: pending
area: core
priority: medium
---

# Tighten Fqn and name-field ergonomics

## Description

Across `beans-core/src/jvm/`, identity-bearing strings are used in
multiple shapes:

- `Fqn(String)` newtype — semantic identity for types/methods/fields.
- `name: String` on `JvmMethodKey`, `JvmFieldKey`, `JvmDeclHeader` — bare
  short names alongside an `owner: Fqn`.
- `TypeRef::Simple { name: String }` — type names at the source level,
  including inside `JvmMethodKey::param_types`.

Each of these clones by full `String` copy; each is unvalidated; each
sits on a hot path (every dynamic link to a method invocation hashes a
key containing several of these strings).

The committed-to deferred work is "swap `Fqn`'s storage to `Arc<str>` or
an intern table" — but that alone does not help the bare-`String` fields
in keys, and it does not address validation.

This item covers a single deliberate design pass before bytecode-level
consumers come online:

1. Decide the cheap-clone story (Arc<str>, intern table, or per-field
   choice). One target shape, not five paper-cuts.
2. Decide whether short non-FQN names should fold into `Fqn` so each
   key carries one identity-string instead of two (e.g.,
   `JvmMethodKey { method: Fqn, param_types: ... }` where `method` is
   `"com.example.Service.process"` rather than `owner` + `name`).
3. Add debug-only validation in `Fqn` constructors (non-empty,
   well-formed identifier-shaped between dots, no leading/trailing
   dots).

## Context

Surfaced during code review of step 3 of the graph migration. The
committed implementation defers `Fqn` interning behind a doc-comment
commitment; the review noted that the commitment by itself does not
remove the cost on the surrounding `name: String` fields.

The Arc-swap on `Fqn` alone reduces clone cost on FQNs but leaves every
key still cloning short names per registration. ADR-0008's "millions of
links" cost model bites at all of these together, not at `Fqn` alone.

## Acceptance criteria

- A single design pass landing the agreed cheap-clone shape across all
  hot-path string fields: `Fqn`, key `name` fields, `TypeRef::Simple`.
- Validation in `Fqn` constructors that catches malformed inputs in
  debug builds.
- Public API stable for downstream consumers (the type names stay; only
  internal storage and ergonomics change).
- No measurable regression in microbenchmarks; ideally a clone-cost
  improvement on the registry hot path.

## Notes

Best done before `beans-jmod` lands (whichever step that is, post-
migration), because the bytecode reader will produce these strings
in volume and any ergonomics decision should account for its load.
