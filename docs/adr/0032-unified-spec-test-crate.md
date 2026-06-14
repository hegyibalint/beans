# ADR-0032: Unify spec and interop tests in one beans-spec-tests crate

## Status

Accepted. Supersedes [ADR-0022](0022-per-language-test-crates-mirroring-spec-structure.md).

## Context

ADR-0022 organized tests as one crate per language (`beans-test-java`,
`beans-test-kotlin`, …) plus a separate `beans-test-interop` crate, each
keyed to a `beans-lang-*` crate so the dependency graph would forbid a
Java test from reaching Kotlin internals. In practice only the Java
vertical exists, so that structure shipped as a single awkwardly named
`beans-lang-java-test` crate sitting next to `beans-lang-java`.

Two assumptions behind ADR-0022 no longer hold:

- **Tests run through the facade, not the vertical.** The fixture harness
  drives the composed `beans` facade (per-extension dispatch, the union
  payload, the composed registries) — not `beans-lang-java` directly. The
  "crate boundary forbids cross-language reach" argument is moot: the
  facade already sees every enabled language, and that is exactly the
  surface a product test should exercise.
- **The product is cross-language.** Beans' reason to exist is behavior
  *across* language boundaries (ADR-0004, ADR-0030): Kotlin nullability at
  a Java use site, a Java override of a Kotlin declaration, a Groovy
  dynamic call into Java. A per-language crate layout makes those tests
  homeless — they are forced into one language's crate or into a separate
  interop crate that has to re-declare dependencies on several verticals.
  Interop is the main event, not an annex.

The per-crate model also multiplies maintenance: every new language would
add both a `beans-lang-X` and a `beans-test-X` crate, and contributors
would keep re-litigating "is this Java-only or interop?" at the crate
boundary.

## Decision

One workspace crate, `beans-spec-tests`, owns all facade-level spec and
interop behavior tests. It depends on `beans` (default features off, the
language features it needs on) and `beans-test-harness` — never on
`beans-lsp` (ADR-0002/ADR-0020 keep the LSP a leaf).

The chapter-mirroring layout from ADR-0022 is kept; only the packaging
collapses. Tests are grouped by area, one test binary per area:

```
beans-spec-tests/tests/
    java.rs              # binary: Java spec by JLS chapter + fix behavior
    java/
        jls04_types.rs … jls15_expressions.rs
        fixes.rs
    interop.rs           # binary: cross-language behavior
    interop/
        kotlin_java/     # Kotlin producer, Java consumer
        java_kotlin/     # the reverse
        groovy_java/ …
    prelude.rs           # shared fixture() + directional-naming notes
```

Interop folders are named `<producer>_<consumer>`: `kotlin_java/` is a
Kotlin producer (declares the symbol) consumed from Java (the use site
under test). The asserted use site always belongs to the consumer
language. `kotlin.rs` / `jvm.rs` and their trees are added as those
verticals land; we do not commit empty test binaries ahead of them.

Vertical-local tests stay put. `beans-lang-java/tests/` remains the home
for parser/model/unit checks that do not need the composed facade. This
ADR is about product/spec behavior, not a mandate to centralize every
unit test.

The testing *disciplines* are unchanged and still apply here:
`expected_failure` bookmarks (ADR-0024), the dual-mode trivial-passer
check (ADR-0025), and its per-test opt-out (ADR-0026).

## Consequences

**Positive.**

- Interop has an obvious, first-class home keyed by direction. The
  canonical cross-language cases stop being homeless.
- One crate to name, build, and reason about. New languages add a
  `beans-lang-X` vertical and a `tests/X/` area, not a whole test crate.
- Tests exercise the surface consumers actually use — the facade — so a
  passing test reflects real product behavior, not a vertical in
  isolation.
- The "executable spec" framing from ADR-0022 survives intact: the
  directory tree still reads as the spec, now with interop alongside.

**Negative.**

- The crate boundary no longer enforces language isolation. A Java spec
  test *could* import a Kotlin fixture by mistake. We accept this: the
  facade is meant to see all languages, and reviewer attention plus the
  dual-mode check carry the weight the crate boundary used to.
- One test crate recompiles as a unit. Touching a shared fixture rebuilds
  the whole spec suite rather than one language's. Acceptable at current
  scale; revisit if compile times bite.
- Area assignment still needs judgment (which JLS chapter, which interop
  direction). That judgment moves from "which crate" to "which folder" —
  cheaper to get wrong and to fix.

## Alternatives considered

**Keep ADR-0022's per-language crates.** Rejected: the crate-boundary
isolation argument no longer holds once tests run through the facade, and
interop — the product's whole point — has no natural home in a
per-language layout.

**One crate, but flat files (no chapter/area subdirectories).** Rejected
for the same reason ADR-0022 rejected it: Java alone spans many chapters;
a flat tree is unreadable and hides coverage gaps. The subdirectory
breakdown is essentially free and keeps the suite legible.

**One crate, organized by feature (`completion/`, `resolution/`, …).**
Rejected per ADR-0003 and ADR-0022: feature-first layout disconnects the
suite from the spec and biases it toward what we implemented rather than
what the spec requires.
