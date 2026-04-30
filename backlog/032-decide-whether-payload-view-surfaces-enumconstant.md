---
status: pending
area: lsp
priority: low
---

# Decide whether payload-view surfaces `EnumConstant` distinctly from `Field`

## Description

The Java walker emits `JavaNodePayload::EnumConstant` for enum
constants (correctly, per JLS §8.9.1). Two LSP-shape views collapse
this back into `SymbolKind::Field`:

- [`payload_view`](../beans-lsp/src/actor.rs) in `beans-lsp/src/actor.rs`
- [`view_fields`](../beans-test-harness/src/fixture.rs) in `beans-test-harness/src/fixture.rs`

Both intentionally map `JavaNodePayload::EnumConstant` to
`SymbolKind::Field` to keep the 338 spec tests stable — those tests
were written against the prototype walker that emitted enum constants
as fields and assert `SymbolKind::Field` for `RED`, `GREEN`, etc.

As a result, the `SymbolKind::EnumConstant` arms in
[`symbol_kind_to_lsp`](../beans-lsp/src/actor.rs) and the kind-string
match in [`build_hover`](../beans-test-harness/src/fixture.rs) are
unreachable in current code paths but remain defensible because
`SymbolKind::EnumConstant` is a real variant of `jvm::SymbolKind`.

The decision: should the LSP-shape views surface `EnumConstant` as
its own kind?

## Pros of distinguishing

- Truthful to the JLS — enum constants are not regular fields.
- Lets clients style enum constants differently (e.g., outline icons,
  hover text).
- The walker's payload distinction stops being a fiction.

## Pros of collapsing (the status quo)

- Spec tests don't churn.
- LSP wire types may not have a distinct concept; clients accustomed
  to `Field` for enum constants may not benefit.
- Smaller mental surface for downstream consumers (CLI tools, custom
  integrations) that just want "field-like things."

## Acceptance criteria

- Project owner picks one shape (or a configurable per-consumer
  shape).
- If "distinguish": `payload_view` / `view_fields` map to
  `SymbolKind::EnumConstant`; spec tests asserting `SymbolKind::Field`
  for enum constants update to `SymbolKind::EnumConstant`; LSP wire
  output adjusts (likely `LspSymbolKind::ENUM_MEMBER` instead of
  `FIELD`).
- If "collapse": delete the `SymbolKind::EnumConstant` arms (which are
  unreachable) and document the decision inline; possibly also remove
  the `EnumConstant` variant from `jvm::SymbolKind` (if no consumer
  ever distinguishes it).

## Context

Surfaced during code review of step 7 of the graph migration. The
walker correctly emits `EnumConstant` after step 7's lockstep fix; the
view-shape-side question is what to do about that.
