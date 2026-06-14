# Architecture Decision Records

This directory holds **Architecture Decision Records** (ADRs) for beans. Each
ADR captures one significant design decision: the context that forced the
decision, what was chosen, the consequences, and the alternatives that were
considered and rejected.

## Why ADRs

Reference docs (`ARCHITECTURE.md`, code comments) describe **what is**. ADRs
describe **why**. Without ADRs, rationale lives in chat logs and PR
descriptions, where it rots and disappears. With ADRs, rationale is durable —
new contributors can understand why the architecture is shaped the way it is,
and old decisions can be revisited with full context when assumptions change.

If a decision is later reversed, write a new ADR (with status `Superseded`)
that points at the old one. The old ADR stays as historical record.

## Format

Each ADR is a numbered Markdown file: `NNNN-kebab-case-slug.md`.

Numbering is monotonically increasing — when adding an ADR, use the next
available number. **Never reuse numbers**, even for ADRs that were rejected
or superseded.

Use this template:

```markdown
# ADR-NNNN: <Short imperative title>

## Status

Accepted | Proposed | Superseded by ADR-XXXX | Deprecated

## Context

What forces drove this decision? What constraints did we have to satisfy?
What was the problem we were solving? Frame this so a reader who wasn't in
the original conversation can understand why this decision was even on the
table.

## Decision

What was decided. State it plainly. One paragraph or a short list.

## Consequences

What follows from this decision. Include both positive and negative
consequences honestly. Future contributors should not be surprised by
downstream costs.

## Alternatives considered

What else was on the table? For each alternative, briefly explain why it
was rejected. This is the most important section for revisiting decisions
later — if assumptions change, you want to know which alternatives might
become viable.
```

## Conventions

- **Title**: short, imperative, focused on the decision (not the topic).
  Good: "Use Rc<RefCell> for registry interior mutability." Bad: "Registry
  storage."
- **Status**: most ADRs are `Accepted`. Use `Proposed` for ADRs under active
  discussion. Use `Superseded by ADR-XXXX` when retiring; never delete the
  old file.
- **Length**: 1-3 pages. ADRs should be tractable to read in one sitting.
  If an ADR is growing past that, it likely covers multiple decisions and
  should be split.
- **Links**: link to other ADRs by number (`ADR-007`). Link to source files
  by relative path. Don't link to chat logs or external sites that may
  disappear.
- **Code examples**: small, illustrative. Don't paste large code blocks; the
  ADR is about the decision, not the implementation.

## Index

ADRs are listed below in numerical order. The "topic" tags help find ADRs
by area.

<!-- Update this index whenever adding an ADR. -->

| # | Title | Topic |
|---|-------|-------|
| [0001](0001-cohesive-not-extensible.md) | Be cohesive, not extensible | foundation |
| [0002](0002-library-first.md) | Build a library, ship an LSP | foundation |
| [0003](0003-spec-drives-implementation.md) | Treat the spec as the source of truth, not the code | foundation |
| [0004](0004-per-language-models-with-jvm-projection.md) | Per-language models with a shared JVM projection | foundation |
| [0005](0005-sync-core-rayon-parallelism.md) | Sync core with rayon for parallelism, no async runtime | concurrency |
| [0006](0006-hard-links-and-dynamic-links.md) | Distinguish hard links and dynamic links in the graph | graph |
| [0007](0007-nodeid-runtime-only-identity.md) | Use NodeId as runtime-only identity; semantic identity lives in registry keys | graph |
| [0008](0008-fallback-queries-on-dynamic-links.md) | Carry an ordered list of fallback queries on each dynamic link | graph |
| [0009](0009-push-stale-pull-recompute.md) | Use push-stale plus pull-recompute for invalidation | graph |
| [0010](0010-lazy-recomputation.md) | Recompute lazily on pull, never eagerly on stale-mark | graph |
| [0011](0011-stable-vs-volatile-nodes.md) | Distinguish stable nodes from volatile nodes | graph |
| [0012](0012-typed-per-registry-keys.md) | Use typed per-registry keys, not a shared key enum | registries |
| [0013](0013-registries-store-all-providers.md) | Registries store all providers; precedence is a resolution concern | registries |
| [0014](0014-raii-handles-for-subscriptions-and-providers.md) | Use RAII handles for subscriptions and provider registrations | registries |
| [0015](0015-rc-refcell-registry-with-weak-handles.md) | Registries are `Rc<RefCell<_>>` with `Weak` back-references in handles | registries |
| [0016](0016-pipeline-phases-before-registration.md) | Run pipeline phases before registration; do not use event-driven processors | lifecycle |
| [0017](0017-no-central-pipeline-machinery.md) | No central pipeline machinery; each node type owns its enrich | lifecycle |
| [0018](0018-single-threaded-graph-core.md) | The graph core is single-threaded; parallelism lives at the file batch | concurrency |
| [0019](0019-single-core-crate-with-feature-gated-languages.md) | Collapse the workspace into a single beans-core crate with feature-gated language modules | crates |
| [0020](0020-lsp-is-a-leaf-consumer.md) | Keep beans-lsp a leaf consumer of beans-core | crates |
| [0021](0021-preserve-tree-sitter-walker-rewrite-layers-above.md) | Preserve the tree-sitter walker; rewrite the layers above it | migration |
| [0022](0022-per-language-test-crates-mirroring-spec-structure.md) | Organize tests as per-language crates mirroring the spec structure (superseded by 0032) | testing |
| [0023](0023-mass-author-spec-tests-via-llm-agents-with-human-review.md) | Mass-author spec tests via LLM agents with human review | testing |
| [0024](0024-tests-start-expected-failure-and-prefer-negative-spec-violations.md) | Tests start as expected_failure and prefer negative spec violations | testing |
| [0025](0025-dual-mode-check-real-engine-vs-empty-engine.md) | Dual-mode check (real engine vs empty engine) catches trivial-passers | testing |
| [0026](0026-per-test-opt-out-for-absence-dependent-tests.md) | Per-test opt-out for absence-dependent tests in the dual-mode check | testing |
| [0027](0027-slim-graph-defer-recomputation-to-layer-2.md) | Limit the graph to a hard-link forest; lazy recomputation lives in layer-2 consumers | graph |
| [0028](0028-stale-while-revalidate-posture.md) | Stale-while-revalidate is the default posture | foundation |
| [0029](0029-layer-1-ir-declarations-and-use-sites.md) | The layer-1 IR contains declarations and use sites, partitioned by modifiability | graph |
| [0030](0030-vertical-crates-engine-jvm-model-language-verticals.md) | Split the workspace into engine, shared JVM model, and per-language vertical crates | crates |
| [0031](0031-one-jvm-sidecar-as-late-arriving-data-pipe.md) | One JVM sidecar as a late-arriving data pipe | tooling |
| [0032](0032-unified-spec-test-crate.md) | Unify spec and interop tests in one beans-spec-tests crate (supersedes 0022) | testing |

## Authoring workflow

1. Pick the next ADR number.
2. Create `NNNN-slug.md` using the template above.
3. Fill in all four sections honestly. The `Alternatives considered` section
   is not optional — if you genuinely had no alternatives, write that.
4. Add an entry to the index in this README.
5. Open a PR. ADRs go through code review like any other change.

## When NOT to write an ADR

- For minor implementation details (which crate a private function lives in,
  naming conventions for variables, etc.) — these belong in code review.
- For decisions that are forced by external constraints with no real choice
  (e.g., "we have to use serde because nothing else has the ecosystem we
  need" — unless the alternatives were genuinely considered).
- For decisions that aren't expected to be revisited.

If you're unsure, err on the side of writing the ADR. Future contributors
will thank you.
