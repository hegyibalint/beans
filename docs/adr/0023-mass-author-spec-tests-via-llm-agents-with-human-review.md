# ADR-0023: Mass-author spec tests via LLM agents with human review

## Status

Accepted

## Context

ADR-0003 commits beans to a spec-driven posture; ADR-0022 organizes the
test suite as an executable specification mirroring the spec structure.
Those decisions imply a *lot* of tests. The Java Language Specification
alone has 18 chapters with hundreds of sections, each describing rules
that need at least a handful of tests to encode. A reasonable density —
say, 20 tests per spec section — puts us in the low thousands of Java
tests, before we even start on Kotlin (~13 chapters), Groovy, Scala
(famously dense), and Clojure.

Hand-writing thousands of tests is not realistic for a project at this
stage. Even with two or three engaged contributors writing nothing but
tests for a year, we would not catch up to the spec. And the rate of
progress matters: if the test suite is always months behind the
specification, the spec-driven posture becomes aspirational rather than
real.

We have, however, a tool that is well suited to drafting structured,
spec-anchored tests: LLM agents. Given a section of the JLS and a
fixture-test idiom to follow, an agent can produce a credible first cut
of the tests for that section. The drafts are not always correct; some
will assert the wrong behavior, miss edge cases, or duplicate each
other. But they are *cheap*, and review is faster than authoring from
scratch.

## Decision

Spec tests are mass-authored by LLM agents and reviewed by humans
before merging. The workflow:

1. An agent is given a spec section (e.g., JLS §15.12 — Method
   Invocation Expressions), the fixture-test idiom from
   `beans-test-harness`, and the per-language test layout from
   ADR-0022.
2. The agent drafts a batch of tests (target ~20 per section, more
   for complex ones, fewer for trivial ones). Each test names the spec
   section, picks a fact stated in the spec, and encodes it.
3. A human reviewer reads the batch, fixes wrong assertions, removes
   duplicates, splits compound tests, and merges what remains.
4. Tests start marked `expected_failure` (see ADR-0024) so the merge
   does not depend on the engine being able to satisfy them yet.

We accept **volume over precision** in the initial pass. A test suite
with some wrong tests is more useful than a test suite with no tests.
Wrong tests reveal themselves at implementation time: the engine
behaves correctly but the test fails, the reviewer rereads the spec,
and the test is fixed. Each test is small and self-contained, so the
cost of fixing one is bounded.

The human reviewer's job is to ensure each batch is *plausibly correct*
and *spec-anchored*, not exhaustively verified. Anything beyond that
slows the pipeline below the rate the project needs.

## Consequences

**Positive.**

- The test suite can grow at the rate the spec demands, not the rate
  hand-authoring permits. We can plausibly reach broad spec coverage
  in months rather than years.
- The agent-author / human-review split plays to each side's
  strengths. Agents are good at generating structured drafts at
  volume; humans are good at sanity-checking and catching the
  occasional confidently wrong assertion.
- The test suite becomes a forcing function for spec literacy.
  Reviewers must read the cited section to evaluate the draft; over
  time the team's collective spec knowledge ratchets up.
- Wrong tests are bounded liabilities. Each test is a few lines;
  fixing one when implementation reveals the error is a small,
  isolated change.

**Negative.**

- Some merged tests will be wrong. We are explicitly choosing to
  accept this rather than gate every test on full verification. The
  cost shows up later as "this test is wrong, fix it" PRs during
  implementation. We believe the amortized cost is lower than
  delaying tests until each is fully verified.
- Reviewer fatigue is a real risk. Reading 20 agent-authored tests
  per section, batch after batch, is taxing. We mitigate by rotating
  reviewers across languages and chapters and by tooling that surfaces
  obvious patterns (duplicated assertions, missing spec citations).
- The agent's drafting style can drift. We mitigate with concrete
  examples in the prompt and periodic audits of recent batches.
- "Volume over precision" is easy to abuse — to merge slop and call
  it progress. The discipline is held by the human reviewer; if
  reviewers stop pushing back, the strategy degenerates. We treat
  reviewer rigor as a non-negotiable, not a nice-to-have.

## Alternatives considered

**Hand-write every test.** Treat every test as a deliberate, human-
authored artifact. Rejected on throughput grounds. At a generous rate
of 10 tests per engineer-hour for spec-anchored fixture tests, a
single language would consume person-years of work. We don't have
person-years; we have months. Hand-authoring is also not obviously
higher quality at this scale — engineers reading specs at 11pm produce
the same kind of mistakes agents do, just slower.

**Generate tests but skip review.** Run the agent at scale, merge the
output, fix tests as implementation finds problems. Rejected because
the failure mode is worse: agent-authored tests have a non-trivial
rate of confidently wrong assertions (asserting the *opposite* of
what the spec says, with full conviction). Without review, those
become load-bearing wrong assumptions in the suite. Reviewing catches
the high-confidence errors cheaply. Skipping review is false economy.

**Generate from the spec automatically (no agent involvement).** Use
a template-based generator that walks the spec text and emits
boilerplate tests. Rejected because spec text is prose; mechanical
extraction produces tests that are syntactically plausible but
semantically empty ("the spec mentions class declaration → here's a
class declaration test"). The agent's value is judgment about which
facts in the spec are testable and how to construct a fixture that
exercises them.

**Delay all tests until features are implemented.** Write a feature,
then tests for that feature. Rejected because it inverts the spec-
driven posture (ADR-0003): the implementation defines the test scope,
not the spec. Tests authored after the implementation tend to encode
"what we built" instead of "what the spec requires," and the gap
between the two is exactly the bug surface we want the test suite to
catch.
