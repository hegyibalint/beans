# ADR-0025: Dual-mode check (real engine vs empty engine) catches trivial-passers

## Status

Accepted

## Context

ADR-0024 establishes the discipline that tests must assert specific
facts the engine has to compute, not the absence of complaints from a
stub engine. The discipline is necessary but not sufficient: it is
enforced by reviewer attention, and reviewer attention is finite.
Especially in mass-authored batches (ADR-0023), tests slip through
that look like meaningful assertions but in fact pass against an
engine that does nothing useful.

The category we want to catch automatically is the *trivial-passer*:
a test that goes green against the real engine but would *also* go
green against an engine that returned nothing for every query. Such
a test isn't testing the engine — it is testing that two empty things
are equal.

The shape of this is mechanically detectable. If we can construct a
controlled "empty engine" — one whose parsers return nothing, whose
registries are empty, whose queries return nothing — then any test
that passes in *both* the real engine mode and the empty engine mode
is, by definition, not exercising the engine. The two modes form a
contrast: a meaningful test should pass in real-engine mode and *fail*
in empty-engine mode, because the empty engine cannot produce the
specific fact the test asserts.

This dual-mode comparison is too expensive to run on every local
`cargo test` invocation — it doubles the suite — but it is exactly
the kind of check CI is for.

## Decision

The harness gains an "empty engine" mode behind a Cargo feature flag.
In empty-engine mode:

- Parsers register no symbols (return empty trees / empty symbol
  lists).
- Registries are empty; lookups always miss.
- Queries return empty results.
- Diagnostics are never emitted.

The CI pipeline runs the test suite **twice**:

1. Once against the real engine. Tests must pass (or be marked
   `expected_failure`, see ADR-0024).
2. Once against the empty engine. Tests must **fail** (the empty
   engine cannot produce the specific facts the tests assert).

A test that passes in *both* modes is a trivial-passer and CI flags
it. The author either rewrites the test to assert something real or
marks it with the per-test opt-out (ADR-0026) if the test legitimately
depends on absence.

Local development runs only the real-engine mode. The dual-mode check
is a CI gate, not a local one — speed of iteration matters more
locally than dual-mode coverage.

## Consequences

**Positive.**

- Trivial-passers cannot land silently. A reviewer who misses the
  pattern in code review still catches it at CI time, with a precise
  failure message ("this test passes against an empty engine, which
  means it isn't exercising real behavior").
- The discipline of ADR-0024 has automated backing. Authors learn
  the norm by hitting the gate; reviewers don't have to be the only
  defense.
- The check is honest. It doesn't try to detect "trivial" via
  heuristics on the test source; it constructs the actual contrast
  and runs it. False positives are bounded to genuine absence-
  dependent tests, and those have an explicit opt-out (ADR-0026).
- Local development stays fast. The doubling cost is paid once per
  CI run, not once per `cargo test`.

**Negative.**

- CI runtime roughly doubles for the test suite. We accept this; the
  test suite is the spec's enforcement mechanism and it's worth the
  CI minutes.
- The "empty engine" must be maintained in lockstep with the real
  engine. As the engine adds new query types, the empty engine
  must learn to say "nothing" for them. We treat this as a one-line
  obligation per new query and review it as part of the query's
  PR.
- Some genuinely correct tests fail dual-mode by their nature
  (absence-dependent tests). These need ADR-0026's opt-out, and the
  opt-out is a maintenance surface we have to keep small.
- A motivated author can defeat the check by making both modes
  produce the same output (e.g., a test that asserts on a string
  literal that doesn't depend on engine behavior). Dual-mode is not
  a substitute for code review; it raises the floor, not the
  ceiling.

## Alternatives considered

**Rely only on the discipline (ADR-0024).** Trust authors and
reviewers to avoid trivial-passers. Rejected because the failure
mode is invisible: a trivial-passer looks like a normal passing
test, and once it lands it will not be revisited. Over thousands of
tests this guarantees a steady accumulation of dead weight that
makes the suite less and less meaningful while looking more and more
comprehensive. The dual-mode check is the cheapest mechanism we
found to make the failure mode visible.

**Mutation testing.** Run the suite against deliberately broken
versions of the engine (e.g., flip a comparison, drop a clause) and
check that the suite fails. Rejected for now as significantly more
expensive (mutating an engine of this size is a dedicated tooling
investment) and not obviously better at catching trivial-passers
specifically — most mutations break enough things that the suite
fails for unrelated reasons. We may revisit if the dual-mode check
proves insufficient.

**Static analysis on test source.** Pattern-match for "absence-only"
tests in the AST of the test code. Rejected because the patterns are
too varied to enumerate (no diagnostics, empty list, equality with
empty struct, etc.) and because the same pattern can be either
trivial or meaningful depending on what the engine does upstream.
Source-level checks are a heuristic; dual-mode is a measurement.

**Always-on dual-mode (local + CI).** Run both modes on every
`cargo test`. Rejected because local iteration speed matters; dev
loop friction is a real cost and we'd rather pay it at CI granularity.
If specific authors want stronger local checks, they can enable the
feature flag locally; we don't impose it.
