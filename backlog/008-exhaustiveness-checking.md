---
status: pending
area: core
priority: medium
---

# Implement sealed-type switch exhaustiveness checking

## Description

For a `switch` statement or expression on a sealed type, determine whether
the case labels exhaustively cover the permitted subtypes. Report a
diagnostic for inexhaustive switches in switch expressions and pattern
switches; classic switch statements opt in via `default`.

Algorithm:

1. Collect the transitive permits closure of the switch selector type.
2. For each case label, mark the matched type and any of its transitive
   permits subtypes as covered.
3. Check that every leaf in the permits closure is covered.
4. Account for record patterns (which require their components to also be
   exhaustive).

## Context

Depends on `RelationKind::Permits` (landed; see backlog 001). The
permits closure is queried via `SupertypeRegistry` per ADR-0012.

Needed for Java 17+ sealed-type pattern matching. Also feeds completion —
when a pattern switch is incomplete, the IDE should offer to add the
missing cases.

## Acceptance criteria

- A switch on `sealed Shape permits Circle, Square` with cases for both
  is exhaustive; missing one is not.
- A switch on a sealed hierarchy with a non-sealed subtype requires a
  `default` to be exhaustive.
- Record-pattern exhaustiveness recurses correctly into components.
- Fixture tests cover the switch-expression case (compile error on
  non-exhaustive) and the pattern-switch case (warning + completion fix).
