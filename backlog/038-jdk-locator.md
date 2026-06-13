---
status: pending
area: core
priority: medium
---

# JdkLocator: discover installed JDKs

## Description

A discovery utility that finds JDK installations on the machine and
reports `(path, version)` pairs, so consumers (the LSP on `initialize`,
a future CLI) can offer sensible defaults without the user wiring paths
by hand. Explicit paths remain the primary library API — the locator is
a convenience on top, per the library-first principle.

Scan roots, in rough precedence order:

- `$JAVA_HOME` (always first when set).
- macOS: `/Library/Java/JavaVirtualMachines/*/Contents/Home` and
  `~/Library/Java/JavaVirtualMachines/*/Contents/Home` (or shell out to
  `/usr/libexec/java_home -V`).
- SDKMAN: `~/.sdkman/candidates/java/*`.
- Gradle toolchains: `~/.gradle/jdks/*`.
- Homebrew: `/opt/homebrew/opt/openjdk*/libexec/openjdk.jdk/Contents/Home`.
- Linux equivalents (`/usr/lib/jvm/*`) when we get there.

Version and vendor come from the `release` file at the JDK root
(`JAVA_VERSION`, `IMPLEMENTOR`). A directory without a parseable
`release` file is reported as unknown-version or skipped.

## Context

Split out of [012-jmod-bytecode-reader](012-jmod-bytecode-reader.md)
("locate the active JDK") during planning of
[037-class-container-layer](037-class-container-layer.md). Dev machines
are expected to carry a wide set of JDKs for many purposes; tests for
037 use `$JAVA_HOME` directly and do not depend on this item.

## Acceptance criteria

- On a machine with `$JAVA_HOME` set, the locator returns it first with
  a correctly parsed version.
- SDKMAN- and macOS-installed JDKs on the scan roots are found with
  versions.
- A candidate directory that is not a JDK (no `release` file, no
  `jmods/` or `lib/modules`) is not reported as one.
- The locator never fails the whole scan because one root is
  unreadable.
