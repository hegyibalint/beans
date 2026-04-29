---
status: pending
area: core
priority: high
---

# Implement cross-file type substitution

## Description

Apply type-parameter substitutions across inheritance chains spanning
multiple files. Given a type expression that mentions a type parameter
declared in an ancestor (e.g., `T` from `Map<K, V>` referenced from a
subclass `StringIntMap extends Map<String, Integer>`), produce the
substituted `TypeRef` with all variables replaced by the corresponding
arguments at each chain link.

## Context

Required for inherited member resolution (backlog 003), overload
resolution (backlog 005), hover, and signature help. The `TypeRef::substitute`
primitive landed with the model foundation (backlog 001); this item is the
algorithm that composes substitutions through `Relation.type_args` chains.

In the graph architecture, this is invoked during inherited member
materialization and on demand for hover/completion. See ADR-0006 (links)
and ADR-0009 (push-stale invalidation) — invalidations propagate when an
ancestor's signature changes.

## Acceptance criteria

- Given `class A<T> { T get(); }` and `class B extends A<String>`, looking
  up `B.get()` yields a return type of `String`, not `T`.
- Multiple chain links compose correctly (e.g.,
  `class C<U> extends B`, `class B<V> extends A<V>` substitutes through).
- Wildcards and bounded type parameters substitute correctly.
- Fixture tests cover at least: single-link substitution, two-link chain,
  substitution into method parameters, substitution into nested generics.
