# ADR-0026: Per-test opt-out for absence-dependent tests in the dual-mode check

## Status

Accepted

## Context

ADR-0025 introduces a dual-mode check: every spec test runs against
the real engine and against an empty engine, and a test that passes
in both modes is flagged as a trivial-passer. The check is a strong
default, but it has a known false-positive case: tests that
*legitimately* depend on absence.

Consider:

```rust
// The import is missing on purpose. The spec says an unresolved
// type produces a diagnostic, and the test asserts that diagnostic.
fixture()
    .file("App.java", r#"
        package com.example;
        public class App {
            private MissingType x;
        }
    "#)
    .resolve()
        .resolves_to_unresolved("MissingType")
    .run();
```

Against the real engine, this test passes: the engine notes the
unresolved type and produces the expected diagnostic. Against the
empty engine, the test *also* passes — the empty engine produces no
resolution either, and "no resolution" is exactly what the test
asserts. The dual-mode check flags the test as trivial, but the test
is correct: the absence is the assertable fact.

We could rewrite such tests to assert on something positive (e.g.,
the specific diagnostic message text), and in many cases that is the
right move — a more specific assertion is a better test. But not
always. Some absence-dependent tests are about the *absence itself*
being the LSP-visible behavior, and contorting them into positive
assertions sacrifices clarity for the sake of the check.

We need a way to tell the dual-mode check "this test is supposed to
pass in both modes." The marker should be rare, deliberate, and
visible in code review.

## Decision

The fixture framework supports a per-test marker that opts a test
out of the dual-mode check:

```rust
.resolve()
    .resolves_to_unresolved("MissingType")
    .absence_dependent("missing import: absence is the assertable fact")
.run();
```

The marker:

- Takes a required justification string. Reviewers see the reason in
  the diff; "because the check failed" is not an acceptable
  justification.
- Excludes the test from the empty-engine pass. The real-engine pass
  still runs normally.
- Is rare by design. Most spec rules are constraints (ADR-0024), and
  most constraint tests assert specific diagnostics rather than
  absence. The marker should appear in a small minority of tests.

The dual-mode check (ADR-0025) tolerates marked tests passing in both
modes; unmarked tests passing in both modes remain a CI failure.

We treat the count of marked tests as a metric we want to keep low.
A spike in `absence_dependent` markers in a PR is a review prompt:
"are these genuinely absence-dependent, or is the author working
around the check?"

## Consequences

**Positive.**

- The dual-mode check stays accurate. False positives have a release
  valve, and the release valve is explicit and visible.
- Absence-dependent tests stay readable. We don't force the author
  to invent an awkward positive assertion just to satisfy the check.
- The marker is a forcing function for review attention. Adding the
  marker is a small, conspicuous diff that prompts the reviewer to
  evaluate whether the absence is genuinely the point.
- The justification string compounds over time into a small
  catalogue of legitimate absence patterns, useful for future
  authors deciding whether their case fits.

**Negative.**

- Authors will sometimes use the marker as an escape hatch when the
  real fix is to write a better assertion. Reviewer rigor is the
  defense; the marker is not a free pass. We accept that some
  marker uses will be wrong and treat marker drift as a code-smell
  to call out in review.
- Two-class tests (with and without the marker) is slightly more
  surface area than one-class tests would be. We judge this
  acceptable: the empty-engine check is high-value enough that
  preserving it is worth one extra concept.
- A marked test that is *also* trivial in the real-engine sense
  (asserts something the engine is not actually computing) cannot
  be caught by the dual-mode check. These will have to be caught
  in code review or by future tooling. The marker explicitly trades
  a small amount of automated coverage for the ability to write
  honest absence-dependent tests.

## Alternatives considered

**Rewrite absence-dependent tests as positive assertions.** Force
every test to assert something concrete, even when "concrete" means
"the diagnostic message contains the word 'unresolved'." Rejected
because the rewrites are often awkward — the assertion drifts from
the spec's intent to whatever incidental fact happens to be
positive. "Missing import produces an unresolved diagnostic" reads
clearly; "the diagnostic for missing-import contains a substring
that the engine happens to include" reads as ceremony. Where a
positive assertion is natural, authors should still prefer it; the
opt-out is for the cases where it isn't.

**Accept the false positives in dual-mode.** Let absence-dependent
tests trip the check, and require reviewers to read each failure
and confirm "yes, this is a legitimate absence-dependent test."
Rejected because it degrades the dual-mode check's signal: every CI
failure becomes a thing to triage rather than a thing to act on.
Once a fraction of failures are "expected," the check stops being a
hard gate and becomes another source of noise.

**Per-file or per-module opt-out.** Apply the marker at a coarser
granularity (e.g., a whole `mod` of unresolved-import tests).
Rejected because the marker should be as narrow as possible. A
file-level opt-out invites scope creep — new tests get added to the
file and silently inherit the opt-out, without anyone evaluating
whether the new test is genuinely absence-dependent. Per-test forces
the question on every test that needs the marker.

**Implicit detection.** Try to auto-detect absence-dependent tests
from the assertion shape (e.g., any assertion that mentions
"unresolved" or "no diagnostics"). Rejected because the heuristics
are unreliable and the failure modes are silent: a test that
*looks* absence-dependent but is actually a trivial-passer would
slip through, and a test that is genuinely absence-dependent but
phrased in an unusual way would still trip the check. Explicit
markers are honest; implicit detection is brittle.
