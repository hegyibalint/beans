---
status: pending
area: lsp
priority: low
---

# Adapt classpath-driven JVM payloads to completion items

## Description

[`beans-lsp/src/completion.rs`](../beans-lsp/src/completion.rs)'s
`to_completion_item` adapter currently handles only
`NodePayload::Java(...)`. For `NodePayload::Jvm(...)` payloads it
falls through silently — the resulting `CompletionItem` has empty
`return_type`, empty `params`, and an empty `detail` string. No path
feeds JVM-only payloads to completion today, so this is harmless;
when classpath-driven completion lands (stdlib methods, third-party
JARs read by a future `beans-jmod` reader per backlog #012) the
adapter needs proper Jvm arms.

## Acceptance criteria

- The adapter handles `NodePayload::Jvm(JvmNodePayload::Method(_))`
  with the same `(paramTypes) -> returnType` formatting as the Java
  arm. Note JVM types are *erased* (per ADR-0012's producer
  obligation on `JvmMethodKey`), so the rendered `detail` may differ
  from the Java arm's pre-erasure form. Decide and document whether
  to render the erased shape (truthful to bytecode) or to lift the
  generic signature back from the JVM's `Signature` attribute (when
  classpath readers expose it).
- The adapter handles `JvmNodePayload::Field(_)` and
  `JvmNodePayload::Constructor(_)` symmetrically.
- `JvmNodePayload::Type(_)` returns an empty `detail` (no source-
  level type signature to render).
- The unit test grows three cases mirroring
  `beans-lsp/src/completion.rs`'s Java tests: a JVM method (erased
  param types render with `[]`-shaped arrays), a JVM field (the
  erased field type renders), a JVM constructor.
- A backlog-cross-reference comment in the existing Java arm names
  this file: `// JVM payloads handled in backlog #034 — see when
  classpath readers go in.`

## Context

Surfaced during code review of step 8 of the graph migration. The
adapter shipped Java-only because (a) every consumer today emits
Java payloads through `parse_java_to_graph`, and (b) classpath
readers don't exist yet. The `// TODO: AnnotationElement` line in
the catch-all of the current adapter is a related stub — same
class of "shipping the contract first, filling in arms when their
producers arrive."

## Related

- Backlog #012 — JMOD bytecode reader (the future producer of
  classpath-loaded `JvmNodePayload`s).
- Backlog #032 — `EnumConstant` view-shape decision.
- Backlog #018 / #030 — Java parser correctness (shares the
  pre-erasure / post-erasure boundary the adapter sits on).
