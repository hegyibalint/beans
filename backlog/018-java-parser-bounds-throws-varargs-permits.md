---
status: pending
area: java
priority: high
---

# Parse Java type-parameter bounds, throws, varargs, permits, non-sealed

## Description

Extend the Java parser to populate the model fields that the foundation
model added but the parser does not yet emit:

- Type-parameter bounds: `<T extends Comparable<T>>` produces a
  `TypeParam` with the upper bound, not just the name.
- `throws` clauses: `void foo() throws IOException` populates
  `Method.throws` with a `Vec<TypeRef>`.
- Varargs: `String... args` sets `MethodParam.is_varargs = true`. The
  varargs marker affects overload resolution and must be distinguished
  from a regular array parameter.
- `permits` clauses: `sealed class S permits A, B` produces relations
  with `RelationKind::Permits`.
- `non-sealed` modifier: parses to `Modifier::NonSealed`.

## Context

These are the most direct consumers of the foundation model fields
that landed in backlog 001. The parser items do not depend on each
other and can land in any order.

Required for: overload resolution (backlog 005), exhaustiveness
checking (backlog 008), and proper handling of inherited members from
generic supertypes (backlog 003).

## Acceptance criteria

- A parsed generic method has `type_parameters` with bounds populated.
- A parsed `throws` clause has the declared exception types as `TypeRef`.
- A parsed varargs method's last parameter has `is_varargs = true`.
- A parsed sealed type has `RelationKind::Permits` relations to each
  named permitted subtype.
- A parsed `non-sealed` declaration has `Modifier::NonSealed`.
- One fixture test per feature, covering both presence and absence.
