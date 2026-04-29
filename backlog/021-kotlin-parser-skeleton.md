---
status: pending
area: kotlin
priority: medium
---

# Add Kotlin language module skeleton

## Description

Create the Kotlin language module as a feature-gated submodule of
`beans-core` per ADR-0019. The module:

- Wires up `tree-sitter-kotlin` (or the chosen grammar — verify it
  handles current Kotlin syntax adequately).
- Provides a parser entry point that walks the tree and produces node
  payloads for the JVM projection per ADR-0004 (per-language model with
  shared JVM projection).
- Registers itself with the engine so files matching `*.kt` flow through
  the Kotlin pipeline.

Initial scope is the structural shapes only: package declarations,
imports, classes (incl. data, sealed, object, companion object),
interfaces, functions (top-level, member, extension), properties,
type aliases. Expression-level resolution is out of scope for this
item — it lands in follow-up items.

## Context

ADR-0019 places the module inside `beans-core` behind a `kotlin`
feature flag. ADR-0004 says the Kotlin model is its own type, and a
projection layer maps it onto the shared JVM model where appropriate
(so a Java caller can resolve a Kotlin class).

This is the foundation item; subsequent Kotlin items (extension function
resolution, `inline`/`reified`, smart casts, etc.) layer on top.

## Acceptance criteria

- `cargo build --features kotlin` succeeds.
- A simple Kotlin file produces nodes for its top-level declarations.
- A Java fixture can resolve a Kotlin class by FQN through the JVM
  projection.
- A regression fixture covers at least: package + import + class with
  one method + one property.
