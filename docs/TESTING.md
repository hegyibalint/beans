# Testing

Testing is a fundamental aspect in Beans; not only because testing is important, but because a lot of the things Beans handles are not made here; the languages, the build tools, and anything external are not developed by Beans. Therefore, if we want to make sure we understand what we are dealing with, we need tests.

## Types

Beans follows the classical testing vocabulary:
 - **Unit**: in-module white-box testing
    - E.g. reaching into internals normally not available on the API
 - **Integration**: testing composed modules
    - E.g. driving `lang-java` and seeing if we resolve the right thing
 - **Acceptance**: testing or preparing for end-to-end behavior
    - E.g. after giving the consumer-facing `Beans` a file and asking for diagnostics, do we get the right answer?
    - E.g. when reading the JLS, already coding down our expectations about resolution, even before implementation

## Where a fact belongs

Test a rule where the code decides it. Higher up, test only that the pieces are connected, not the rule again.

Java says a member type wins over a single-type import (JLS §6.4.1). The `crates/lang-java/src/resolution.rs` is the code that decides it, so that is where the cases belong: the member type alone, the import alone, both together, and the import written first and written last. That module is the only one that has to be exhaustive.

The levels above it can still be wired wrong, so each of them gets one test; not another case, just the connection:

| Test | What it establishes |
|---|---|
| `resolution/tests/shadowing.rs` | every case: which declaration wins |
| `lang-java/tests/type_resolution/shadowing.rs` | one: the answer survives the crate's public API |
| `acceptance/.../type_resolution/shadowing.rs` | one: the answer reaches a user of Beans |

Without this, tests drift upward. The level furthest from the code is the most comfortable one to write in; the fixture is nice, the setup is familiar, and nothing stops one more case from going there. The result is that the top restates rules the bottom already owns, and a single broken rule fails in three places at once, without telling us which one is wrong.

A fact moves down when the code that decides it moves down; it does not move up as it matures.

## Levels

Tests in Beans are highly structured. This is necessary because this piece of software touches multiple, complex domains, and tests have to stay discoverable, maintainable and scalable.

To do this, the following system is used as a shared vocabulary:

| Level | Names | Example |
|---|---|---|
| Domain | the broad area under test | `languages`, `tools`, `engine` |
| Subject | the outside thing we test against | `java`, `java_kotlin`, `maven` |
| Capability | what Beans does, observably | `navigation`, `type_resolution` |
| Premise | what must hold for the case to make sense | `shadowing`, `multi_module` |
| Claim | the fact the test establishes | `missing_type_is_an_error` |

The levels are ordered, and only the claim is mandatory. Most tests carry just a few of them, and a level is missing for one of two reasons:

 - **There is nothing to name.** `engine/` has no subject, because what it tests is Beans itself. There is no outside thing to put there.
 - **It hasn't grown yet.** The level is real, but the cases still fit in one file, so it hasn't earned a directory. A few tests written inline in a module can all be about `shadowing` while no `shadowing` directory exists anywhere: the premise is in the tests, just not in the path yet.

The difference matters when a test is added. A level with nothing to name stays missing forever, so don't invent one. A level that hasn't grown is where the next directory appears, once the file gets uncomfortable.

## Structure

Placing a test is two questions: which tree does it go in, and how deep.

The tree comes from the type. Each of the three has its own home, and its own Cargo rules:

| Type | Lives in | Can reach |
|---|---|---|
| Unit | next to the module, under `src/` | everything, including private items |
| Integration | a crate's `tests/`, next to its `src/` | that crate's public API |
| Acceptance | `tests/acceptance/tests/` | the `Beans` facade |

A test can live in one file until that file gets uncomfortable. Then, the file becomes a directory, and the level below it becomes the new file.

When to split is subjective:
 - when the testing starts to become cumbersome
 - when there are multiple concepts mixing in the same file
 - when it is _expected_ that multiple concepts will mix
    - e.g. it's obvious that `resolution.rs` will contain a complex, multi-faceted codebase; we can start with the split structure right away.

### Unit tests

Unit tests live with the module they exercise, but not necessarily in the same file. They have two shapes, and which one we are in decides how much of the test the path can carry.

We follow the same fact through both shapes: a member type wins over a single-type import. The `crates/lang-java/src/resolution.rs` decides it, so this is where its cases belong.

#### Embedded

The smallest shape is an inline `#[cfg(test)] mod tests { ... }` block, right in the module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_type_shadows_single_type_import() { ... }
}
```

Only the claim has anywhere to go:

| Level | Value |
|---|---|
| Domain | None |
| Subject | None |
| Capability | None |
| Premise | None |
| Claim | `member_type_shadows_single_type_import` |

```text
resolution.rs::tests::member_type_shadows_single_type_import
```

Grouping the cases into child modules buys back the premise without moving a single file, giving `resolution::tests::shadowing::member_type_shadows_single_type_import`. That only holds while the whole block is still comfortable to read.

#### Externalized

When the block gets uncomfortable, it moves out into a `tests.rs` index plus a `tests/` directory holding one file per premise. What never happens is the middle state where `tests.rs` collects the test functions itself: that file is an index, never a container.

`resolution.rs`'s tests fall into five premises: resolution through lexical scope, through the same package, through an import, at an occurrence in a body, and where two declarations compete for one name. They get five files:

```text
resolution.rs
resolution/
├── tests.rs
└── tests/
    ├── lexical.rs
    ├── same_package.rs
    ├── imports.rs
    ├── occurrence.rs
    └── shadowing.rs
```

Then the `resolution.rs` declares `tests` as a testing-only module:

```rust
#[cfg(test)]
mod tests;
```

The `tests.rs` declares the child modules and holds the fixture helpers shared by more than one of them, but no `#[test]` functions itself. A helper used by a single premise stays in that premise's file.

```rust
mod imports;
mod lexical;
mod occurrence;
mod same_package;
mod shadowing;

fn identifier(text: &str) -> JavaIdentifier { ... }
```

Now the premise has a file to live in, so it leaves the function name:

| Level | Value |
|---|---|
| Domain | None |
| Subject | None |
| Capability | None |
| Premise | `shadowing` |
| Claim | `member_type_shadows_single_type_import` |

```text
resolution/tests/shadowing.rs::member_type_shadows_single_type_import
```

Nothing above premise shows up in either shape. A unit test already sits in one module of one crate, so a capability directory would just repeat what the module path says. That is a different kind of absence: the premise was missing until it grew, but the capability stays missing however large the file gets.

### Integration tests

Integration tests sit in a crate's `tests/` directory, next to its `src/`. Cargo compiles each as its own crate, so they reach the crate under test only through its public API, making the tests black-box by design.

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

The files don't follow the module names. `navigation` is not a module, it's a capability. Integration tests cross modules by definition, so the capability is the name that actually fits the group.

Splitting works the same as everywhere else, only the index is a `main.rs`, because Cargo links whatever sits directly under `tests/` as its own binary. Here `type_resolution` has grown and `navigation` has not, so they sit side by side as a directory and a file:

```text
crates/
└── lang-java/
    ├── src/
    │   └── ...
    └── tests/
        ├── navigation.rs
        └── type_resolution/
            ├── main.rs
            ├── single_type_import.rs
            └── shadowing.rs
```

An integration test necessarily composes things from the outside: for example, `navigation.rs` builds a `PlatformJvm` and drives revisions through `core`. What makes it an integration test is where the attention is: if the assertions focus on establishing the behavior of `lang-java`, and the external components are just there to make things work, it can be considered an integration test.

If the attention is on the overall system behaving, i.e. the assertions spread around, the test is an acceptance test rather than an integration test.

The same fact gets one test here, checking that `lang-java`'s public API still gives the right answer. It is not a second copy of the cases; those stay with the code that decides them. The test spans modules, so the group needs a name that isn't a module name, and that is the capability:

| Level | Value |
|---|---|
| Domain | None |
| Subject | None |
| Capability | `type_resolution` |
| Premise | `shadowing` |
| Claim | `member_type_shadows_single_type_import` |

```text
lang-java/tests/type_resolution/shadowing.rs::member_type_shadows_single_type_import
```

There is nothing to name above capability here. The crate directory already says the domain and the subject, so putting them inside `tests/` would say it twice.

### Acceptance tests

Acceptance tests live in the `tests/acceptance` package and drive the `Beans` facade as a black box. The fixture lives in that package's `src/`; the claims live in its `tests/`.

Acceptance tests skip the user-facing facets of Beans and use it as a library instead, driving the `engine` crate. They are effectively one more facet: instead of serving a user, this one asserts behavior.

That is also why acceptance tests never drive the LSP. The LSP is one facet among several; a CLI or an embedding tool forks off at the same level. So the behavior has to be right at the engine surface already. The facets above then consume a library that is already well tested, and their own tests can focus on translation and transport alone.

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
    │   │   ├── dependencies.rs
    │   │   └── source_sets.rs
    │   └── gradle/
    │       └── ...
    └── engine/
        ├── main.rs
        └── staleness/
            └── replacement.rs
```

This is the only tree wide enough to need a domain:

 - `languages/` tests our assumptions about the languages we support. For example:
   -  The `java/` subject tests the behavior we expect Beans to give based on the JLS; 
   -  The `interop/` subject tests cross-language behavior, in `<producer>_<consumer>`.
 - `tools/` tests tooling interactions, for example:
   - Reading a `pom.xml` or a Gradle build into a classpath, a module path, and a source set.
 - `engine/` tests what Beans invented itself. For example storage or indexing.

Each domain is one Cargo test target rooted in a `main.rs`, because Cargo compiles and links whatever sits directly under `tests/` as its own binary.

Below a subject, tests are grouped by capability, just like integration tests. `type_resolution/` is a directory only because it holds ninety-five tests; `maven/dependencies.rs` stays a single file until it can't.

Inside a grown capability, each file collects the cases that share a premise: something that has to hold for the case to make sense at all.

 - `type_resolution/single_type_import.rs`: there is a single-type import
 - `type_resolution/shadowing.rs`: two declarations compete for one name
 - `dependencies/multi_module.rs`: the project has several modules

A test belongs to the file whose premise is the point of the test. When a test needs two premises, the interaction wins: `member_type_shadows_single_type_import` needs both a member type and an import, but what it establishes is precedence rather than the behavior of either side, so it lives in `shadowing.rs`.

One last test for the same fact sits here, checking that it reaches a user of Beans. This tree holds every language and every tool, so it finally has to say which one we are in: the subject, and the domain above it.

| Level | Value |
|---|---|
| Domain | `languages` |
| Subject | `java` |
| Capability | `type_resolution` |
| Premise | `shadowing` |
| Claim | `member_type_shadows_single_type_import` |

```text
acceptance/tests/languages/java/type_resolution/shadowing.rs::member_type_shadows_single_type_import
```

That is the last level to arrive, and the two kinds of absence are now visible next to each other. The premise was missing in the embedded shape only until it grew; one shape later it appeared, without the test itself changing. Domain, subject and capability were missing because there was nothing to name at that reach, and no amount of growth would have added them; only testing the fact at a further reach did.

Once a test has a file of its own, that file always turns out to be the deepest level above the claim. And this is not only a filing scheme: minus the leading directories, the same path is what `cargo test` prints and what you pass to filter a run.

#### Specification tests

Some acceptance tests are written before the behavior exists: we read the JLS and code down what resolution should do. These are not a separate tree. They are ordinary acceptance tests, filed by the same levels, marked as pending with `expected_failure`:

```rust
.resolves_to("cursor", "com.example.Foo")
.expected_failure("single-type imports are not resolved yet")
```

The mark turns the usual cycle around. A pending expectation is green while it fails, and turns the suite **red the moment it starts passing**. That is the signal to drop the mark. So the suite tells us when the implementation caught up with the spec, instead of us having to spot it.

The citation rules are still open: when a test should cite a JLS section, and what that citation lets us change later.

## Naming

Name a test function after the claim it establishes, not after the code it calls. `parses_compilation_unit_declarations` names a method and tells us nothing about what should be true; `declarations_expose_their_names_and_name_spans` names a fact we can agree or disagree with. A test whose claim is hard to phrase is usually a test establishing more than one thing.

Since the path is read as a whole, the enclosing modules carry part of the sentence and the function should not repeat them. `lexical::prefers_the_innermost_scope` says everything `lexical_resolution_prefers_the_innermost_scope` said. Sometimes avoiding the repetition costs more in readability than it saves; `shadowing::member_type_shadows_single_type_import` is one of those, so keep it.

Don't prefix with `test_`; `#[test]` already says that. Avoid `basic`, `works`, and `simple`: they name our confidence, not the behavior.
