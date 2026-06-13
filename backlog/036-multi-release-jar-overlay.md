---
status: pending
area: core
priority: low
---

# Resolve multi-release jar overlays in the class container layer

## Description

Multi-release jars (JEP 238) overlay base classes with per-release
variants under `META-INF/versions/<N>/`. The class container layer
(jmod/jar enumeration, split out of
[012-jmod-bytecode-reader](012-jmod-bytecode-reader.md)) currently
enumerates base entries only and ignores `META-INF/versions/` entirely.

Scope:

- Container takes a target release (e.g. 21) when opening a jar.
- Gate on the manifest: `Multi-Release: true` must be present in
  `META-INF/MANIFEST.MF`, otherwise `META-INF/versions/` is ignored
  (per spec).
- Effective view: for each class, the highest versioned entry `<=`
  target release wins; base entry otherwise. Consumers never see
  `META-INF/versions/` paths.

## Context

Deferred from the first container-layer deliverable: the JDK jmods have
no MRJ semantics, and most versioned variants swap implementation
rather than API surface, so base-only enumeration is correct enough for
resolution in the common case. This item closes the gap for jars where
the API surface does differ by release (e.g. classes that exist only in
the versioned tree).

## Acceptance criteria

- Opening an MRJ with target release N returns the versioned variant
  for overlaid classes and the base entry for everything else.
- A jar with `META-INF/versions/` entries but no `Multi-Release: true`
  manifest attribute enumerates base entries only.
- A class present only under `META-INF/versions/<M>` (M <= N) appears
  in the effective view; one with M > N does not.
