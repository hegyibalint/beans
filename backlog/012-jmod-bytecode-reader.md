---
status: pending
area: core
priority: high
---

# Implement classfile decoder and JDK/jar graph materialization

## Description

Read JVM bytecode into the node/registry model so external library
symbols (the JDK and dependency jars) participate in resolution
alongside source symbols.

Two slices were split out during planning:
[037-class-container-layer](037-class-container-layer.md) (archive
access — enumerate classes in `.jmod`/`.jar`, hand out raw bytes) and
[038-jdk-locator](038-jdk-locator.md) (find JDK installs). This item is
what remains:

- Decode classfile bytes: constant pools, class structure, methods,
  fields, attributes (Signature for generics,
  RuntimeVisibleAnnotations for annotations, PermittedSubclasses for
  sealed types, Record for record components).
- The decoder takes bare `&[u8]` with no container coupling — class
  bytes also arrive from loose files (build output directories), so
  this layer must stand free of 037.
- Materialize each class as node payloads with hard links for nested
  classes and dynamic links for type references resolved through registries.

The output should be a "stable" node set per ADR-0011 — JMOD-derived nodes
do not change between sessions, so they can be serialized and warm-loaded.

## Context

This is the bridge between source code and the rest of the JVM ecosystem.
Without it, references to `String`, `List`, and every other JDK type fail
to resolve.

ADR-0019 places this code in `beans-core` as a sibling of the language
modules (not behind a language feature flag — it serves all languages).
The CLI-only build (default features off) still includes the JMOD reader.

## Acceptance criteria

- Given a JDK install, the reader produces nodes for `java.lang.Object`,
  `java.util.List`, etc. with correct supertype and member links.
- Generic signatures decode correctly (e.g., `List<E>`'s type parameter).
- Sealed/record/annotation attributes populate the corresponding model
  fields.
- A fixture test resolves a Java source reference to `java.lang.String`
  through the JMOD reader.
- Cold-load time is measured and recorded; an item to add warm-load
  serialization can be filed separately if needed.
