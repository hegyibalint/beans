---
status: pending
area: core
priority: medium
---

# Implement SAM (functional interface) detection

## Description

For an interface, determine whether it is a functional interface — i.e.,
has exactly one abstract method after applying inherited member resolution
and ignoring methods overriding `Object`'s public methods (`equals`,
`hashCode`, `toString`).

Output: the resolved abstract method (with substituted signature) when the
interface is a SAM, otherwise a clear "not a SAM" outcome with the reason
(zero abstract methods, multiple abstract methods, etc.).

## Context

Required for lambda type inference, method-reference resolution, and
`@FunctionalInterface` validation. Depends on inherited member resolution
(backlog 003) — SAM detection runs on the materialized abstract member set.

Note that the JLS rule is "exactly one abstract method that is not a public
method of `Object`," and inherited default and static methods do not count.
Re-abstraction (a sub-interface that re-declares an inherited abstract
method) does not increase the count.

## Acceptance criteria

- `Runnable`, `Function<T, R>`, `Comparator<T>` are detected as SAMs.
- An interface with two abstract methods is not a SAM.
- An interface whose only abstract method overrides `Object.equals` is not
  a SAM.
- An interface that extends another SAM and adds only default methods is
  still a SAM, with the correct abstract method.
- Fixture tests cover the `@FunctionalInterface` annotation validation
  case (a non-SAM annotated `@FunctionalInterface` produces a diagnostic).
