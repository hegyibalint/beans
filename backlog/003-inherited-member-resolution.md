---
status: pending
area: core
priority: high
---

# Implement inherited member resolution

## Description

Walk the supertype DAG of a class or interface and collect inherited members
(fields and methods), applying JLS visibility, hiding, and override rules.
For each candidate member produce the resolved declaration along with the
substitution from the declaring type's type parameters to the requesting
type's arguments.

Specifically:

- Traverse `extends` and `implements` (including `Object` for classes).
- Honor visibility — package-private members from a different package are
  not inherited.
- Apply hiding (static-static), overriding (instance-instance), and
  re-abstraction rules.
- Report ambiguity when two unrelated supertypes contribute incompatible
  members of the same erased signature.

## Context

This is the foundation algorithm for member completion, override checking,
SAM detection, and synthetic-member generation. Depends on `TypeRef`,
`TypeParam`, and `Relation.type_args` (all landed; see backlog 001).

In the new architecture (ADR-0006, ADR-0012, ADR-0013) the inputs are read
through `SupertypeRegistry` and `SymbolRegistry`; the algorithm registers
its result as a node payload that consumers subscribe to.

## Acceptance criteria

- Given a class with a superclass and one or more interfaces, the algorithm
  returns the union of accessible members with correct origin metadata.
- Hidden members are not returned; the hiding member shadows them.
- Overridden methods are returned only as the overrider's declaration.
- Type substitution is correctly applied (e.g., `List<String>.add` returns
  the parameter type as `String`, not `T`).
- Fixture tests cover at least: simple inheritance, diamond, generic
  inheritance with substitution, package-private exclusion, interface
  default methods.
