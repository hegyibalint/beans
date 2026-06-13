# 036 — Sidecar v1: core + gradle/import + root-driven indexing

Status: pending
Priority: high
Depends on: beans-toolchains (done)
Design: ADR-0031

## Scope

1. **`beans-sidecar/` Gradle project** (in-repo, not a Cargo member):
   - `core`: stdio JSON-Lines loop, request dispatch, progress
     notifications, handshake with per-duty capability report,
     `WorkspaceModel` schema types.
   - `gradle`: `gradle/import` duty via the Tooling API with stock
     models (`IdeaProject`/`GradleProject`); produces `WorkspaceModel`
     (source/test/generated roots, compile classpath, module deps,
     jdkHome per module). Fat jar output.
2. **`beans-sidecar` Rust crate** (client):
   - JVM selection via `beans-toolchains` (`min_major: 17`).
   - Spawn/restart-with-backoff/shutdown lifecycle; request
     correlation; typed `gradle/import`; workspace sniffing
     (`settings.gradle*`).
   - Jar discovery: setting → `beans-sidecar/build/libs/` dev path →
     alongside-binary.
3. **LSP wiring**: eager-background spawn at initialize for
   Gradle-shaped workspaces; import result ingested as a late wave.
4. **The v1 consumer — root-driven indexing**: replace the blind
   workspace walk with `WorkspaceModel` roots when available
   (fall back to the walk otherwise); index generated source roots
   (AP level 0). Classpath stored dormant for #012.

## Acceptance

- /tmp playground variant with a real Gradle build: opening the
  workspace spawns the sidecar, import completes in background,
  generated-source types (e.g. a checked-in AP output dir) resolve.
- Killing the sidecar mid-import leaves beans serving; restart policy
  fires; status surfaced.
- No JVM on PATH/host: beans behaves exactly as today.

## Out of scope (future duties, same protocol)

`maven/import`, `sbt` (likely BSP client), `ap/run` (in-process
`-proc:only`; needs custom tooling model + #012), Lombok simulation
(beans-lang-java enrichment, delombok as oracle — its own item).
