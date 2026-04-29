---
status: pending
area: core
priority: medium
---

# Enforce access control during resolution

## Description

Filter members during resolution by their declared accessibility relative
to the access site:

- `public` — always accessible.
- `protected` — accessible to subclasses (even from another package) and
  to all members of the same package.
- package-private (no modifier) — accessible only within the same package.
- `private` — accessible only within the same top-level class (including
  nested members of that class).

Apply this in completion (filter inaccessible members), in resolution
(reject inaccessible references with a diagnostic), and in inherited
member resolution (do not include inaccessible members in the inherited
set).

## Context

Depends on `Modifier` (landed; see backlog 001) and `PackageRegistry`
(per ADR-0012). The access-site context — declaring class, declaring
package — is part of the request to the resolution algorithm.

## Acceptance criteria

- A `private` field of `A` is not visible from `B` even when `B extends A`.
- A `protected` member is visible from a subclass in another package only
  through that subclass (the JLS "qualifying type" rule).
- A package-private class in `com.foo` is not accessible from `com.bar`.
- Diagnostics are produced for explicit references to inaccessible
  members; completion silently filters them.
- Fixture tests cover each visibility level and at least one cross-package
  case.
