---
status: pending
area: core
priority: low
---

# Detect ambiguous wildcard imports

## Description

When a compilation unit has two or more wildcard imports (`import a.*;`
`import b.*;`) that contain types of the same simple name, an unqualified
reference to that simple name is a compile error. Detect this situation
and produce a diagnostic at the reference site.

Single-type imports always win over wildcard imports (and over types in
the same package), so the algorithm runs only after single-type imports
have been considered.

## Context

Depends on `PackageRegistry` (queried per ADR-0012). Each wildcard import
becomes a fallback query on the dynamic link from the reference to the
imported types (ADR-0008).

Low priority: this is a rare case in practice (most projects do not use
wildcard imports for unrelated packages), but javac and IDEs do flag it,
so beans should match.

## Acceptance criteria

- A reference to `Date` with both `java.util.*` and `java.sql.*` imported
  produces an "ambiguous reference" diagnostic.
- A single-type import for `Date` resolves the ambiguity (no diagnostic).
- Fixture test covers the ambiguous case and the single-type-resolved
  case.
