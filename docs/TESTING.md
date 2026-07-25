# Testing

Testing is a fundamental aspect in Beans; not only because testing is important, but because a lot of the things Beans handles are not made here; the languages, the build tools, and anything external are not developed by Beans. Therefore, if we want to make sure we understand what we are dealing with, we need tests.

## Types

Beans follows the classical testing vocabulary:
 - **Unit**: in-module white-box testing
    - E.g. reaching into internals, like when stripping package names
 - **Integration**: testing composed modules
    - E.g. driving `lang-java` and seeing if we resolve the right thing
 - **Acceptance**: testing or preparing for end-to-end behavior
    - E.g. after giving the consumer-facing `Beans` a file and asking for diagnostics, do we get the right answer?
    - E.g. when reading the JLS, already coding down our expectations about resolution, even before implementation

## Structure

To keep tests scalable and maintainable, we follow these structure and naming rules in Beans.

### Unit tests

Unit tests live with the module they exercise, but not necessarily in the same file. They have exactly two shapes, and nothing in between.

A module could either contain its tests inline, in an `#[cfg(test)] mod tests { ... }` block, or, as they become larger or cover more than one subject, break them out into a `tests.rs` index plus a `tests/` directory holding one file per subject. There is no middle state where `tests.rs` collects everything: that file is an index, never a container.

Take `crates/lang-java/src/resolution.rs`. Its tests fall into four subjects, so they get four files:

```text
resolution.rs
resolution/
├── tests.rs
└── tests/
    ├── lexical.rs
    ├── same_package.rs
    ├── imports.rs
    └── occurrence.rs
```

`resolution.rs` declares `tests` as a testing-only module:

```rust
#[cfg(test)]
mod tests;
```

`tests.rs` is a simple index. It declares the child modules and holds the fixture helpers shared by more than one of them, but no `#[test]` functions itself. A helper used by a single subject stays in that subject's file.

```rust
mod imports;
mod lexical;
mod occurrence;
mod same_package;

fn identifier(text: &str) -> JavaIdentifier { ... }
```

The split pays for itself in naming: the subject moves into the module path, so it comes out of the function name.

```text
lexical_resolution_prefers_the_innermost_scope    →  lexical::prefers_the_innermost_scope
same_package_ignores_a_type_from_another_package  →  same_package::ignores_a_type_from_another_package
```

When to move from inline to the broken down structure is subjective:
 - when the module testing starts to become cumbersome
 - when there are multiple concepts mixing in the same module
 - when it is _expected_ that multiple concepts will mix
    - e.g. it's obvious that `resolution.rs` will contain a complex, multi-faceted codebase; we can start with the `tests/` solution right away.

### Integration tests

Integration tests sit in a crate's `tests/` directory, next to its `src/`. Cargo compiles each as its own crate, so they reach the subject only through its public API, making the tests black-box by design.

```text
crates/
└── lang-java/
    ├── src/
    │   ├── lib.rs
    │   ├── model.rs
    │   ├── parser.rs
    │   └── resolution.rs
    └── tests/
        └── navigation.rs
```

The test files don't need to follow the module names. Note that `navigation` above is not a module, it's a capability. We should strive to make integration tests capability-named, as they will probably test cross-module functionality.

If a suite starts to outgrow a single file, we can use a structure similar to unit tests, only with a `main.rs` becoming the root that declares all the other files as modules.

```text
crates/
└── lang-java/
    ├── src/
    │   └── ...
    └── tests/
        └── navigation/
            ├── main.rs
            ├── cross_file.rs
            └── shadowing.rs
```

An integration test necessarily composes things from the outside: for example, `navigation.rs` builds a `PlatformJvm` and drives revisions through `core`. What makes it an integration test is where the attention is: if the assertions focus on establishing the behavior of `lang-java`, and the external components are just there to make things work, it can be considered an integration test.

If the attention is on the overall system behaving, i.e. the assertions spread around, the test is an acceptance test rather than an integration test.

### Acceptance tests

Acceptance tests live in the `tests/acceptance` package and drive the `Beans` facade as a black box. The fixture lives in that package's `src/`; the claims live in its `tests/`.

Acceptance tests skip the top facet layers of Beans, and rather use Beans as a library. The tests drive the `engine` create, sort-of mimicking an extra facet to Beans: one that instead doing something user facing, testing behavior. 

The top facets then consume a library that is already well tested, and they can focus their testing on integration with the engine alone. 

Looking from the very top, acceptance tests are located in `tests/acceptance/`:

```text
tests/acceptance/
├── src/
│   └── ... # Fixtures and utilities
└── tests/
    ├── languages/
    │   ├── main.rs
    │   ├── java/
    │   │   └── type_resolution/
    │   │       ├── scope.rs
    │   │       ├── single_type_import.rs
    │   │       ├── shadowing.rs
    │   │       └── ...
    │   └── interop/
    │       └── java_kotlin/ # Java declarations, consumed from Kotlin
    │           └── ...
    ├── tools/
    │   ├── main.rs
    │   ├── maven/
    │   └── gradle/
    └── engine/
        └── main.rs
```

Just like integration tests, acceptance tests are structured after capabilites rather than project/modules they test:

 - `languages/` tests our assumptions about the languages we support. For example:
   -  The `java/` module tests the behavior we expect Beans to give based on the JLS; 
   -  The `interop/` module tests cross-language capabilities, in `<producer>_<consumer>`.
 - `tools/` tests tooling interactions, for example:
   - Reading a `pom.xml` or a Gradle build into a classpath, a module path, and a source set.
 - `engine/` tests what Beans invented itself. For example storage or indexing.

Each top-level directory is one Cargo test target rooted in a `main.rs`, because every entry directly under `tests/` compiles and links as its own binary.

Below a subject, tests are grouped by capability, the same unit integration tests use. A capability is one file until it outgrows one, at which point it becomes a directory — the same growth as an inline test module moving out into `tests/`. `type_resolution/` is a directory only because it holds ninety-five tests; `tools/maven/dependencies.rs` can stay a single file until it doesn't.

Inside a grown capability, each file collects the cases that share a premise: something that has to hold for the case to make sense at all.

 - `single_type_import.rs` — there is a single-type import
 - `shadowing.rs` — two declarations compete for one name
 - `maven/multi_module.rs` — the project has several modules

A test belongs to the file whose premise is the point of the test. When a test needs two premises, the interaction wins: `member_type_shadows_single_type_import` needs both a member type and an import, but what it establishes is precedence rather than the behavior of either side, so it lives in `shadowing.rs`.

## Naming

Read from the outside in, a test path answers the same questions in every suite:

```text
<domain>/<subject>/<capability>/<premise>.rs::<claim>
```

| Level | Names | Example |
|---|---|---|
| Domain | the broad area under test | `languages`, `tools`, `engine` |
| Subject | the thing being characterized | `java`, `java_kotlin`, `maven` |
| Capability | what Beans does, observably | `navigation.rs`, `type_resolution/` |
| Premise | what must hold for the case to make sense | `shadowing.rs`, `multi_module.rs` |
| Claim | the fact the test establishes | `missing_type_is_an_error` |

Levels collapse when there is only one of something:

 - `languages/java/type_resolution/shadowing.rs::member_type_shadows_single_type_import`
   - Every level is present; an acceptance test in a capability that grew into a directory.
 - `engine/staleness/replacement.rs::reprocessing_a_source_replaces_its_declarations`
   - No subject: what `engine/` characterizes is Beans itself, not an outside language or tool.
 - `crates/lang-java/tests/navigation.rs::resolves_a_cross_file_type_at_the_right_edge`
   - An integration test, so neither domain nor subject; `navigation` is a capability that still fits in one file.
 - `crates/core/src/file.rs::line_starts_follow_every_newline`
   - A unit test still embedded in its module: only the module it belongs to, and the claim.

A `.rs` file is therefore whichever level is the deepest one present — a premise when the capability has grown into a directory, the capability itself when it hasn't.

This is not only a filing scheme: minus the leading directories, the same path is what `cargo test` prints and what you pass to filter a run.

Name a test function after the claim it establishes, not after the code it calls. `parses_compilation_unit_declarations` names a method and tells us nothing about what should be true; `declarations_expose_their_names_and_name_spans` names a fact we can agree or disagree with. A test whose claim is hard to phrase is usually a test establishing more than one thing.

Since the path is read as a whole, the enclosing modules carry part of the sentence and the function should not repeat them. `lexical::prefers_the_innermost_scope` says everything `lexical_resolution_prefers_the_innermost_scope` said. Where avoiding the repetition costs more in readability than it saves — `shadowing::member_type_shadows_single_type_import` — keep the repetition.

Don't prefix with `test_`; `#[test]` already says that. Avoid `basic`, `works`, and `simple`: they name our confidence, not the behavior.