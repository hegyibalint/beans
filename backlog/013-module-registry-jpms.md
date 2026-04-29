---
status: pending
area: core
priority: low
---

# Implement ModuleRegistry for JPMS

## Description

A registry tracking JPMS module declarations — exports, requires, opens,
qualified exports, and transitive requires. Used to enforce module
boundaries during resolution: a class in module `m1` that is not exported
to module `m2` is not visible from `m2`, even if it would be otherwise
accessible by Java access modifiers.

Scope:

- Parse `module-info.java` declarations in source.
- Decode `module-info.class` from JMOD/jar files.
- Build the module graph and resolve transitive requires.
- Hook into resolution so cross-module access is filtered.

## Context

Low priority because most beans users are likely on the classpath, not the
module path; even projects on the module path tend to expose enough that
classpath-style resolution works in practice. This item is what unlocks
correct diagnostics for module-path projects.

ADR-0012 (typed per-registry keys) and ADR-0013 (registries store all
providers) apply directly: a `ModuleRegistry` is one more registry with
its own key type.

## Acceptance criteria

- The registry exposes the module graph: given a module name, return its
  exports/requires/opens.
- Resolution filters out classes that are not exported to the requesting
  module.
- A fixture test sets up two modules where one does not export a package
  to the other and verifies the access is denied.
- `module-info` from JMOD files is decoded and contributes to the graph.
