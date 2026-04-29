---
status: pending
area: groovy
priority: medium
---

# Add Groovy language module skeleton

## Description

Create the Groovy language module as a feature-gated submodule of
`beans-core` per ADR-0019.

Initial scope:

- Package declarations and imports.
- Classes, interfaces, traits, enums, annotation types.
- Methods (with optional return types), fields, properties.
- Closures parsed as expressions (resolution is a later item).

Groovy is dynamic by default but ships a `@CompileStatic` mode and
`@TypeChecked` annotation that downstream tooling cares about. The
parser tracks the annotation but does not yet enforce the type rules
they imply.

## Context

Per ADR-0004 Groovy has its own model with a JVM projection. Groovy
is interesting because Gradle build scripts are Groovy by default, so
this module is on the path for Gradle script support — a likely beans
audience.

The Groovy grammar is permissive (almost everything is optional), which
means the parser produces a lot of node payloads with `Unknown` /
`Inferred` type slots. Resolution items can decide later how aggressive
to be about inferring those.

## Acceptance criteria

- `cargo build --features groovy` succeeds.
- A simple `.groovy` file produces nodes for its top-level
  declarations.
- A Gradle-style script with `apply plugin: 'java'` parses without
  errors and produces nodes for the implicit `script` class.
- A Java fixture can resolve a Groovy class by FQN through the JVM
  projection.
