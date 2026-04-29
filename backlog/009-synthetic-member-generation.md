---
status: pending
area: core
priority: medium
---

# Implement synthetic member generation

## Description

Generate the implicit members that the JLS specifies for certain symbol
kinds, so they appear in completion, resolve via go-to-definition, and
participate in overload resolution as if they were declared in source.

Coverage:

- Enums: `values()` returning `T[]`, `valueOf(String)` returning `T`,
  inherited `name()`, `ordinal()`, etc., from `java.lang.Enum`.
- Records: canonical constructor (if not user-declared), accessor methods
  for each component, `equals`, `hashCode`, `toString`.
- Classes without an explicit constructor: a no-arg public constructor
  matching the class's accessibility.
- Anonymous and local classes: implicit constructors derived from the
  enclosing context.

Synthetic members are produced as node payloads with a flag distinguishing
them from source-declared members so diagnostics can identify them.

## Context

Depends on `Signature::Record`, `RecordComponent`, and `EnumConstant`
(all landed; see backlog 001). Required for completion on records and
enums, and for proper overload resolution when the synthetic constructor
is the target.

In the new architecture (ADR-0017) each language's enrich step produces
the synthetic members; there is no central pipeline that injects them.

## Acceptance criteria

- Enum `Color` exposes `values()` and `valueOf(String)` in completion.
- Record `Point(int x, int y)` exposes `x()` and `y()` accessors and a
  canonical constructor.
- A class with no declared constructor has an implicit one with matching
  visibility.
- Synthetic members are flagged so hover can show "synthetic" provenance.
- Fixture tests cover all four kinds.
