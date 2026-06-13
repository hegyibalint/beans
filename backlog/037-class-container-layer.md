---
status: completed
area: core
priority: high
---

# Class container layer: read classes out of .jmod and .jar

## Description

The archive-reading slice split out of
[012-jmod-bytecode-reader](012-jmod-bytecode-reader.md): open a `.jmod`
or `.jar`, enumerate the classes inside, hand out raw class bytes.
Classfile *decoding* is explicitly not this layer — it stays in 012 and
must accept bare `&[u8]` with no container coupling, because class
bytes also arrive from loose files (build output directories like
`build/classes/`).

Both formats are zip archives of classfiles; only the layout differs:

- `.jmod` — 4-byte magic prefix (`JM` + version `0x01 0x00`), classes
  under `classes/`, non-class sections (`lib/`, `conf/`, `bin/`, …)
  ignored.
- `.jar` — classes at the root, `META-INF/` skipped entirely (which
  also defers multi-release overlays to
  [036-multi-release-jar-overlay](036-multi-release-jar-overlay.md)).

Decisions made during planning:

- **Explicit per-format types** — `Jmod::open()` and `Jar::open()` as
  distinct public types sharing a common accessor surface via a private
  inner; no public trait (ADR-0001: cohesive, not extensible). A future
  consumer needing "either container" uniformly adds an enum wrapper
  then.
- **`zip` crate** for archive access (no default features, `deflate`
  only). This code is unconditional in `beans-core` per ADR-0019, so
  the dependency is paid by every consumer.
- **Binary names** at the API boundary (`java.lang.String`,
  `java.util.Map$Entry`) — container layout is stripped; mapping to the
  model's `Fqn` is a later layer's concern.
- **`module-info.class` is skipped** in enumeration until
  [013-module-registry-jpms](013-module-registry-jpms.md) needs it.
- **Tests run against a real JDK** (`jmods/java.base.jmod`,
  `lib/jrt-fs.jar`); dev machines are assumed to carry JDKs. Amended
  after delivery: instead of trusting `$JAVA_HOME` (which may point at
  a runtime without `jmods/`), tests use `beans-test-jdks` to download
  and cache a pinned Temurin. JDK *discovery* for the library proper is
  still [038-jdk-locator](038-jdk-locator.md).

## Acceptance criteria

- `Jmod::open($JAVA_HOME/jmods/java.base.jmod)` enumerates binary
  names including `java.lang.String` and a nested class
  (`java.util.Map$Entry`); `module-info` does not appear.
- `class_bytes("java.lang.String")` returns bytes starting with
  `0xCAFEBABE`.
- `Jar::open($JAVA_HOME/lib/jrt-fs.jar)` enumerates class names with
  no `META-INF` leakage and no path separators.
- Opening a jar as `Jmod` (or vice versa) fails with a clear magic
  mismatch error, not a corrupt-archive error.
