---
status: pending
area: java
priority: medium
---

# Parse Java records and enum constants correctly

## Description

Two related parser fixes:

- Records: parse `record Point(int x, int y) { ... }` into
  `Signature::Record` with the components in declaration order. Today
  the parser does not produce record components; record declarations
  parse as classes with no special handling.
- Enum constants: parse the constants in an enum body as
  `SymbolKind::EnumConstant`, not `SymbolKind::Field`. Today the parser
  emits them as fields, which is wrong for completion ranking and
  diagnostics.

## Context

Both rely on model elements that already exist (`Signature::Record`,
`SymbolKind::EnumConstant` from backlog 001). The change is purely on
the parser side.

Records also need to compose with synthetic-member generation
(backlog 009) — once parsed correctly, the canonical constructor and
component accessors are produced from the record's declaration.

## Acceptance criteria

- Parsing `record Point(int x, int y)` produces a `Signature::Record`
  with two `RecordComponent` entries in source order.
- Each enum constant in `enum Color { RED, GREEN, BLUE }` parses to a
  `SymbolKind::EnumConstant`.
- Enum constants with bodies (e.g.,
  `RED { @Override String label() { return "red"; } }`) parse with the
  body's members nested correctly.
- Fixture tests cover the record case (incl. an enum constant body
  case) and assert the kinds, not just the names.
