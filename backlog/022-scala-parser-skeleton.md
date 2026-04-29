---
status: pending
area: scala
priority: low
---

# Add Scala language module skeleton

## Description

Create the Scala language module as a feature-gated submodule of
`beans-core` per ADR-0019. Initial scope is structural parsing of
Scala 3 (the dialect we target by default; Scala 2 syntax overlaps
enough that the same grammar handles both for the basic shapes).

Cover:

- Package declarations (incl. nested package syntax).
- Imports (incl. wildcard, renamed, and given imports).
- Classes, traits, objects, enums (Scala 3), case classes.
- Methods and `def`/`val`/`var` definitions.
- Type aliases.

Implicit and given resolution, type-class derivation, and macro support
are out of scope for the skeleton. They land in follow-up items if and
when they are needed.

## Context

Per ADR-0004 Scala has its own per-language model with a JVM projection.
Scala's expression grammar is the most complex of the JVM languages, so
the skeleton deliberately ducks anything that requires full type
inference.

Low priority because Scala usage in our target audience is much smaller
than Java/Kotlin/Groovy, and the cost-benefit of supporting expression-
level Scala correctness is steep.

## Acceptance criteria

- `cargo build --features scala` succeeds.
- A simple Scala file produces nodes for its top-level declarations.
- A Java fixture can resolve a Scala class by FQN through the JVM
  projection.
- A regression fixture covers at least: package + import + class +
  case class + object.
