---
status: pending
area: core
priority: medium
---

# Implement LUB (least upper bound) computation

## Description

Compute the least upper bound of a set of types as defined by JLS 4.10.4.
Required for typing the conditional operator (`? :`), array initializers
with mixed element types, and multi-catch clauses.

The algorithm:

1. Compute the erased candidate set by intersecting supertypes.
2. Compute the minimal candidate set.
3. Compute the parameterization of each candidate by intersecting parameter
   bounds across all input types (lcta — least containing type argument).
4. Return the intersection type, or a single class if only one candidate
   remains.

## Context

Depends on `SupertypeRegistry` and `TypeRef` (landed; see backlog 001).
Used by type inference, hover formatting, and diagnostics on conditional
expressions.

LUB is notoriously full of edge cases (recursive type parameters, raw types
in the input set, the null type). Test against JLS examples directly rather
than rolling intuition.

## Acceptance criteria

- LUB of `String` and `Integer` returns the intersection of their common
  supertypes (`Comparable<? extends ...> & Serializable`, etc.).
- LUB of `List<String>` and `List<Integer>` produces
  `List<? extends Object & Comparable<...> & Serializable>`.
- LUB of `null` and any reference type `T` returns `T`.
- Fixture tests target ternary expressions and array literals from the JLS
  examples for chapter 4.10.
