---
status: pending
area: core
priority: high
---

# Implement JMOD/JAR bytecode reader

## Description

Read JVM bytecode from JDK `.jmod` files and project bytecode jars into the
node/registry model so external library symbols (the JDK and dependency
jars) participate in resolution alongside source symbols.

Scope:

- Locate the active JDK and enumerate its `.jmod` files.
- Read class files out of `.jmod` (zip with a JDK-specific layout) and out
  of plain `.jar` archives.
- Decode constant pools, class structure, methods, fields, attributes
  (Signature for generics, RuntimeVisibleAnnotations for annotations,
  PermittedSubclasses for sealed types, Record for record components).
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
