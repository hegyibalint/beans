# Testing

## Unit testing

Unit tests should be inline in the module they test, not in separate test files declared with `mod tests;`.

Their role is to establish local data-handling correctness and robustness are handled predictably.
For example, we should make sure that we cover:
 - malformed or missing input
 - boundary values
 - unusual paths

Keep these tests small and focused; most of our testing should be integration, not unit testing.

## Integration testing

The main way of testing in Beans.
Tests are located in a sibling `tests` directory in the crate root.
This way, we are forcing ourselves to be a consumer of the code, and not an insider.

Their role is to establish the effects of connected behavior, like:
 - was the right data stored?
 - what happens when an operation is repeated?
 - do cooperating components see each other's changes?

These tests built upon the assumptions tested by unit tests; we don't need to repeat input validation for example.
They should also establish the consequences of failure across boundaries, for example:
 - rejected input must not leave partial writes.
 - a failed operation must not prevent a later valid operation from succeeding where recovery is supported.

This has some effects on API design:
 - Visibility should be thought out in a way that it supports testing, hence observability into modules.
 - Dependencies should be made so they are composable instead of ad-hoc used from the outside.

When writing a new module, struct, or function, think of it's integration test not only testing behavior, but "neatness": well design code is most often elegant to test.
When testing is difficult, examine why before adding workarounds. Propose reshaping APIs and dependency boundaries so behavior is easier to exercise and its effects are easier to observe.

## Shared test utilities

Before adding test setup, look for existing utilities in `beans-testing` and similar setup elsewhere. Reuse what fits.
If an existing utility does not fit, consider improving or reshaping it before adding another variant.
A better design may require migrating existing tests; prefer that work over accumulating overlapping helpers just to keep the current change small.

When multiple suites need the same infrastructure—databases, temporary project trees, file operations, or binary fixtures—keep it in `beans-testing`.
Maintain it as a coherent, discoverable set of utilities, not a pile of alternatives.
Consolidate overlapping helpers and remove superseded ones as needs evolve.
Judge the result by the clarity of the whole testing suite, not only the test being added.

Keep scenario-specific setup and assertions close to their tests.
Share mechanics, not merely code that looks similar; helpers work well, if they make test contracts easier to read by removing unnecessary detail.

A good example of this is `database(...)` where the creation of the database is a need, but not a concern by many databases.
These compose into larger helpers like `engine(...)`; the point is to hide unnecessary and unimportant detail.
