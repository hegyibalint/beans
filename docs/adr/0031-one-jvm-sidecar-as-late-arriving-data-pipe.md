# ADR-0031: One JVM sidecar as a late-arriving data pipe

## Status

Accepted

## Context

Real workspaces are defined by their build tools: source roots (including
annotation-processor output directories), classpaths, module structure,
and toolchains live in Gradle/Maven/sbt models, and extracting them
requires a JVM (the Gradle Tooling API is a Java library; Maven likewise).
Annotation processing, if ever executed by us, also requires a JVM.

Beans' founding posture (ADR-0028) forbids making any of that a
prerequisite: the engine serves from its graph immediately; tooling data
is a *pipe* that fills in whenever it can. A machine with no JVM gets a
working LSP minus dependency awareness, with status surfaced — never a
blocked one.

Two findings from the design sessions shaped the decision:

1. **Edit-time annotation processing is mostly a non-problem.** The best
   IDEs do not run processors on keystrokes. IntelliJ indexes the
   generated-source directories produced by the *build* (freshness =
   last build) and special-cases Lombok by simulating its effects in the
   IDE's model — Lombok is not a generator; it mutates the user's own
   classes inside javac and produces no indexable files. Eclipse/jdtls
   get Lombok via a javaagent patching their ecj, which is inapplicable
   to beans (we run no Java compiler at edit time).
2. **Per-save JVM forking is unaffordable.** Any future on-save
   processor execution must run *in-process* in an already-warm JVM
   (`ToolProvider.getSystemJavaCompiler()`, reused file manager,
   per-module scope), not as a forked `javac` per save.

## Decision

**One sidecar JVM process per workspace**, serving every JVM-bound duty
through duty modules behind a single protocol.

- **Spawn**: eager-in-background at workspace open when the workspace
  smells like a build (`settings.gradle*`, later `pom.xml`/`build.sbt`).
  JVM selected via `beans-toolchains` (`min_major: 17`); absence of a
  JVM degrades to no-sidecar with surfaced status.
- **Protocol**: JSON Lines over stdio, JSON-RPC shaped (`{id, method,
  params}` / `{id, result|error}` plus id-less progress/log
  notifications). Methods namespace by duty: `gradle/import`,
  `maven/import`, `ap/run`. The handshake reports per-duty capability
  (e.g. `ap/run` requires the sidecar to run on a JDK; on a JRE the
  duty reports unavailable while imports keep working).
- **Lifecycle**: crash → bounded restarts with backoff → give up and
  surface status; shutdown by stdin close with kill-after-grace; one
  import in flight per workspace with cancellation; build-file changes
  debounce into re-import. The sidecar is killable by design — duties
  must tolerate restart.
- **Gradle duty v1**: Tooling API with *stock* models (no injection
  into the user's build). The custom tooling model (init-script plugin)
  is the planned evolution, becoming mandatory when `ap/run` lands —
  stock models do not expose the `annotationProcessor` path.
- **The one schema**: every duty produces the same `WorkspaceModel` —
  per module: name, source roots, test roots, **generated source
  roots**, compile classpath, module dependencies, JDK home. The v1
  actionable payload is the roots (indexing becomes build-accurate and
  generated types resolve from the last build); the classpath is
  carried dormant as the bytecode reader's (#012) future work-queue.
- **Annotation processing posture**: index generated roots (now);
  watch/re-index them (cheap follow-up); in-process `-proc:only`
  regeneration in the sidecar is a deferred differentiator gated on
  #012 + the custom tooling model. **Lombok is explicitly not a sidecar
  duty**: it is simulated as a `beans-lang-java` enrichment
  (synthesized member declarations per handler, delombok as the test
  oracle), the IntelliJ approach.
- **Code layout**: `beans-sidecar/` in-repo as a standalone
  single-module Gradle project building one fat jar (split into
  core + per-tool modules when a second build tool lands). Not a
  Cargo member. Rust side: a `beans-sidecar` client crate (spawn, correlate,
  typed duties, workspace sniffing) consuming `beans-toolchains`. Jar
  discovery: setting → dev path (`beans-sidecar/build/libs/`) →
  alongside-binary.

## Consequences

**Positive.**

- One process, one lifecycle, one protocol on user machines; new build
  tools are sidecar modules + a sniff rule, not new architecture.
- The never-block property is structural: every duty result is a late
  wave into the graph through the same ingestion path.
- AP pain is mostly solved without executing processors; the genuinely
  expensive piece (Lombok) lands in the vertical where per-feature
  simulation belongs.
- The sidecar being killable doubles as processor isolation once
  `ap/run` exists.

**Negative.**

- A Java codebase and Gradle build enter the repo; CI builds two
  ecosystems.
- Stock TAPI models cap import fidelity (no AP path, coarse source-set
  detail); a second extraction iteration (custom model) is scheduled,
  not avoided.
- Protocol versioning between the Rust client and the jar is ours to
  manage (single repo keeps them in lockstep for now).
- One sidecar means one blast radius: a wedged duty can stall others
  until the restart policy fires.

## Alternatives considered

**Per-tool sidecars.** Cleaner isolation, N processes/lifecycles on user
machines and N JVMs resident. Rejected by explicit owner decision: one
sidecar.

**`gradlew` + init script printing JSON (no TAPI, maybe no sidecar).**
Cheapest possible; loses daemon reuse, progress, cancellation, and
structured failures; stdout parsing of builds is fragile. Rejected.

**Custom tooling model from day one.** No second migration later, but
plugin jar versioning + injection + compatibility matrix on day one for
fidelity v1 does not need. Deferred, not rejected.

**gRPC / LSP-framing / binary protocols.** Dependency weight or
ceremony without benefit for a local pipe between two components we
both control. Rejected.

**Fork `javac -proc:only` per save.** JVM startup + cold JIT + full
classpath re-read per save. Rejected after cost analysis in favor of
in-process compilation in the warm sidecar (when that duty lands).

**Lombok via delombok shadow sources or agent patching.** Shadow
sources duplicate types in the index; agent patching requires hosting a
Java compiler at edit time, which beans does not do. Rejected in favor
of IR-level simulation.
