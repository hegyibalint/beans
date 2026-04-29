# ADR-0024: Tests start as expected_failure and prefer negative spec violations

## Status

Accepted

## Context

ADR-0023 commits us to mass-authoring spec tests, often before the
engine implements the relevant features. That implies most tests will
fail when first written — there is nothing to make them pass yet. The
fixture framework already supports `expected_failure`, which runs the
test, expects it to fail, and *fails the test if it unexpectedly
passes* (so an unexpected pass becomes a "promote me" signal).

There is a second, subtler problem. The natural way to write a spec
test is often "valid code → no diagnostics, correct resolution":

```rust
fixture()
    .file("Foo.java", r#"
        package com.example;
        public class Foo {}
    "#)
    .resolve()
        .resolves_to("com.example.Foo")
    .run();
```

Against a fully working engine, this asserts something real. Against a
stub engine that does almost nothing, it can pass *trivially*: the
engine returns nothing, the resolution fails to fire any diagnostics,
the assertion sees no contradicting evidence, and the test goes green.
"No diagnostics" is the default state of an engine that is doing
nothing at all.

For a spec-driven test suite, this is the worst kind of bug: tests
that pass without exercising the engine. They give a false sense of
coverage and they survive in the suite indefinitely because nothing
forces them to be looked at again.

Most spec rules are *constraints* — they describe what is forbidden,
malformed, or incorrect. Their natural test form is a violation: feed
the engine code that breaks the rule, and assert the engine produces
the specific diagnostic the spec implies.

## Decision

Two disciplines, applied together.

**Every spec test starts marked `expected_failure`.** No exceptions
during the spec-authoring phase. If the engine actually implements
the feature, the test fails the `expected_failure` check (because it
unexpectedly passes) and the author promotes it by removing the
marker. If the engine doesn't implement it, the marker keeps CI green
while encoding the expectation. The marker is treated as a bookmark
("this is what we owe the spec"), not as a permanent excuse.

**Tests prefer specific assertable facts over absence.** Tests must
assert something the engine has to compute and produce. Concretely:

- "Violation X produces diagnostic Y at position P" — good. Requires
  the engine to detect X and emit Y.
- "Symbol S resolves to FQN F with kind K" — good. Requires the
  engine to perform resolution and report a specific result.
- "This file produces no diagnostics" — bad. An engine that does
  nothing also produces no diagnostics. This passes trivially.
- "This identifier has no resolution" — bad in isolation; see
  ADR-0026 for the legitimate version.

For each spec rule, the author asks: "what is the most specific fact
I can assert that would change if the engine stopped working?" That
is the assertion the test should make. Most spec rules are
constraints, so the natural form is a *violation* test that asserts
the specific diagnostic the constraint produces.

## Consequences

**Positive.**

- The suite can be authored ahead of the engine without churning CI.
  `expected_failure` carries the promise; the engine catches up at
  its own pace.
- Most tests assert something real. When the engine is actually
  exercised, the test must observe a specific computation; an empty
  engine cannot fake that.
- Promotion is a positive signal. When a test stops being
  `expected_failure`, the author has shipped something. The diff is
  small and visible.
- The bias toward *violation* tests aligns with how specs actually
  read. Specs largely describe constraints; tests that exercise
  violations cover the spec's intent more directly than tests that
  exercise the happy path.

**Negative.**

- `expected_failure` can become a wallpaper marker — "I'll deal with
  it later" — applied to tests that should already pass. We mitigate
  by treating unexpected passes as merge-blocking failures (the
  fixture framework already does this) and by periodic audits.
- Some spec rules genuinely require positive assertions (e.g., "this
  *is* a valid construct" — a permission, not a constraint). These
  tests are harder to make non-trivial; ADR-0026 carves out the
  legitimate cases with an explicit opt-out.
- "Prefer violations" is a bias, not a rule. Authors must still
  exercise judgment. Some readers will read this ADR and believe
  every test must be a violation test; that is wrong, and we will
  occasionally need to push back in review.
- The discipline is enforced by reviewer attention. ADR-0025 adds an
  automated check, but it cannot catch every variant of "trivially
  passing." Reviewer rigor remains the primary defense.

## Alternatives considered

**"Valid code produces no diagnostics" tests.** Use happy-path tests
that assert the absence of complaints. Rejected as the canonical
trivial-passer. They pass against an engine that does nothing,
they pass against an engine that does the right thing, and they
pass against an engine in every state in between. The signal-to-
noise ratio is poor. Where positive assertion is genuinely needed,
ADR-0026's opt-out exists.

**Mark tests as "trivially passing" with a separate flag.** Allow
trivial tests but tag them so they're filtered out of the meaningful-
coverage metric. Rejected as ratchet noise: the tag becomes another
piece of ceremony for every test, easy to forget, easy to misapply,
and the tags never get cleaned up. We prefer the simpler rule:
write tests that assert something real, and use `expected_failure`
to bridge the gap between intent and engine.

**Skip tests entirely until the engine implements the feature.**
Write tests in tandem with implementation, never ahead of it.
Rejected because it makes the suite an artifact of implementation
rather than of the spec (see ADR-0003). It also strips the suite of
its planning function: a test marked `expected_failure` is a
commitment to implement; a deleted test is just an absence.

**Trust authors to avoid trivial-passers without an explicit rule.**
Rejected because the failure mode is silent. A test that trivially
passes looks identical to one that meaningfully passes; nothing
about the test code itself reveals the problem. The discipline has
to be explicit so reviewers know what to look for, and the
automated check (ADR-0025) has a stated norm to validate against.
