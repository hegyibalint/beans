# ADR-0022: Organize tests as per-language crates mirroring the spec structure

## Status

Accepted

## Context

Beans targets five JVM languages (Java, Kotlin, Groovy, Scala, Clojure) plus
their interactions on a shared JVM projection. ADR-0003 commits the project
to a spec-driven posture: the language specifications are the source of
truth, and the implementation must answer to them. That commitment forces
a question for the test suite: how do we organize tests so the suite itself
is a faithful reflection of the specs we are claiming to implement?

The naive approach — one big `beans-tests` crate with all language features
mixed together — has been used by other multi-language tools and tends to
collapse under its own weight. Tests cluster by whoever wrote them, not by
what they exercise. Coverage of any single spec is hard to read off the
test list. Cross-language interop tests sit awkwardly next to language-
specific tests with no clear boundary.

We also have a structural reason to keep the language test suites
independent: the per-language Cargo crates already establish a clear
boundary (`beans-lang-java`, `beans-lang-kotlin`, …). Anchoring tests to
those boundaries keeps the dependency graph honest — a Kotlin test cannot
silently depend on Java parsing internals because the crate doesn't see
them.

## Decision

Tests are organized into per-language test crates plus one cross-language
crate:

- `beans-test-java`, `beans-test-kotlin`, `beans-test-groovy`,
  `beans-test-scala`, `beans-test-clojure` — one per language. Each
  depends only on `beans-test-harness`, `beans-core`, and its own
  `beans-lang-*` crate.
- `beans-test-interop` — cross-language scenarios. A Java file consuming a
  Kotlin symbol, a Scala class extending a Java interface, etc. Depends
  on multiple `beans-lang-*` crates.

Within each per-language crate, tests are organized **by spec chapter**:

```
beans-test-java/tests/
    spec/
        jls04_types.rs
        jls06_names.rs
        jls07_packages.rs
        jls08_classes.rs
        jls09_interfaces.rs
        ...
```

Each file maps to a chapter of the language spec (Java Language
Specification, Kotlin spec, etc.). Inside a file, sub-modules map to
sections (`mod jls_7_5_1_single_type_import { ... }`). Each test cites the
spec section it exercises in a comment near the top of the test or via
the module name itself.

The result is a test suite that reads as an executable specification:
opening `beans-test-java/tests/spec/jls08_classes.rs` shows what beans
claims about JLS Chapter 8.

## Consequences

**Positive.**

- The test layout is the spec layout. A reviewer asking "do we cover
  JLS §7.5?" finds the answer by reading the directory tree.
- Per-language crates compile and test independently. A change to
  `beans-lang-kotlin` doesn't force the Java test suite to recompile.
- Coverage gaps are visible. An empty file `jls10_arrays.rs` is a
  louder signal than scattered absences in a monolithic file.
- The crate boundary enforces architectural discipline. A Java test
  cannot reach into Kotlin internals; the crate doesn't see them.
- Interop tests have a natural home (`beans-test-interop`) that is
  obviously not a per-language test. There is no ambiguity about where
  a cross-language test belongs.

**Negative.**

- More crates to maintain. Each new language adds two crates
  (`beans-lang-X`, `beans-test-X`).
- Tests that genuinely span multiple languages live in
  `beans-test-interop`, which means contributors have to decide
  "is this Java-only or interop?" Borderline cases (a Java test that
  happens to import `java.lang` via the JVM projection) need a
  convention; we keep these in the per-language crate unless they
  exercise multi-source interop.
- Spec chapters do not always partition cleanly. A test about a
  generic method might touch JLS §8 (classes), §15 (expressions), and
  §4 (types). We pick the primary chapter and accept some judgment
  calls, the same way the spec authors do.
- Naming conventions must be enforced (`jlsNN_topic.rs`,
  `mod jls_N_M_subsection`). Style drift across contributors is a real
  risk; we mitigate with code review and a linter rule if it becomes
  painful.

## Alternatives considered

**One big test crate with all languages enabled.** Simplest from a
project-structure perspective. Rejected because it conflates the
languages: the dependency graph would let a Java test depend on Kotlin
internals by accident, and the suite stops being a per-language
specification. It also makes incremental compilation of language
suites worse; touching one language's parser rebuilds tests for all
five.

**Organize tests by feature rather than spec chapter.** For example,
`completion/`, `resolution/`, `imports/`, `generics/`. This is closer
to how the LSP feature surface is organized. Rejected because it
disconnects the tests from the spec. Two consequences follow: it
becomes harder to see whether a given spec section is covered, and
the tests start to drift toward what we *implemented* rather than what
the spec *requires*. The whole point of ADR-0003 is to keep the spec
in charge; the test layout should reinforce that.

**Per-language crates but flat test files.** Keep one crate per
language, but put all tests in `tests/spec.rs` without a chapter-
based subdirectory. Rejected because Java alone has 18 chapters and
hundreds of sections; a single file would be unreadable. The chapter
breakdown is essentially free organizationally and makes the
"executable spec" framing legible.
