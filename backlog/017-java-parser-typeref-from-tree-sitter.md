---
status: pending
area: java
priority: high
---

# Parse Java type references as TypeRef

## Description

Replace the string-based type extraction in the Java parser with a parser
that produces structured `TypeRef` values from tree-sitter type nodes.
Covers all Java type expressions:

- Primitive types and `void`.
- Class and interface types with type arguments.
- Array types (including multi-dimensional and varargs).
- Wildcards: `?`, `? extends T`, `? super T`.
- Type variables (references to type parameters).
- Nested types (`Outer.Inner`, `Outer<X>.Inner<Y>`).
- Intersection types in `extends` bounds.

The `beans-lang-java/src/types.rs` file contains the existing primitive
extractor and is preserved per ADR-0021; this item extends it to cover
the full grammar.

## Context

`TypeRef` (landed; see backlog 001) is unused on actual source until
the parser populates it. This item is what connects the foundation
model work to real Java code.

Depends on the parser node-payload migration (backlog 016) being in
flight, but can land function-by-function ahead of the full payload
switch as long as the produced `TypeRef` is stored on the new payload
fields.

## Acceptance criteria

- `Map<String, ? extends Number>` parses to a `TypeRef::Class` with two
  type arguments, the second a wildcard with an upper bound.
- `int[][]` parses to a nested array `TypeRef`.
- `T extends Comparable<? super T>` parses with the right structure.
- A unit test exists for each significant grammar shape and references
  the JLS chapter that defines it.
