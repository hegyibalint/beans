---
status: pending
area: core
priority: high
---

# Implement 3-phase overload resolution

## Description

Implement JLS 15.12.2 method invocation overload resolution across the three
applicability phases: strict invocation, loose (boxing/unboxing) invocation,
and variable-arity (varargs) invocation. For a given call site with argument
types, return the most specific applicable method, or report ambiguity when
no unique most-specific method exists.

Sub-tasks:

- Strict applicability — exact subtype match, no boxing/unboxing.
- Loose applicability — allow boxing, unboxing, and primitive widening.
- Variable arity — match `MethodParam.is_varargs` declarations.
- Most-specific selection across the candidate set.

## Context

Required for resolving call expressions, completion ranking, signature help,
and diagnostics on ambiguous calls. Depends on `TypeRef`, primitive widening,
boxing rules, and `MethodParam.is_varargs` (all landed; see backlog 001),
plus inherited member resolution (backlog 003).

## Acceptance criteria

- Strict phase rejects candidates that need boxing or varargs; loose phase
  accepts them; varargs phase only runs if loose finds nothing applicable.
- Most-specific selection picks `foo(int)` over `foo(long)` for an `int`
  argument, etc.
- Ambiguity is reported (not silently picked) when JLS rules say it should
  be ambiguous.
- Fixture tests cover at least: primitive widening, autoboxing, varargs vs.
  fixed-arity preference, and the classic `foo(Object)` vs. `foo(String)`
  with a `null` argument case.
