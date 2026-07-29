# Wishlist

Claims we want to pin here but cannot express yet.

This is not the same thing as `expected_failure`. That mark is for a claim the
fixture can already state and the engine cannot yet meet, and it turns the suite
red the moment the engine catches up. Everything below is a claim the fixture
cannot state at all, so nothing will tell us when it becomes possible. Hence the
list.

Each entry says what the test would claim, what blocks it, and what unblocks it.

## A compiled artifact in the lake

`Fixture::file` takes `.java` text and runs it through the Java parser, so every
source it creates is a `JvmSource::SourceFile`. Nothing can put a class file, a
jar entry or a runtime image into the class lake, so no acceptance test reaches
the projection side of resolution.

Blocked claims:

- A member type of a class file resolves through its nested binary name.
  `java.util.Map.Entry` is `java.util.Map$Entry` in the lake, and the walk has to
  spell the `$` (case 3 on `resolve_canonical_name`).
- The same, deeper: `p.A.B.C` is `p.A$B$C` (case 4).
- A source file beats a class file of the same binary name, which is what javac's
  `-Xprefer:source` decides.
- The `Jvm` arm of `member_types` at all. Every green case today goes through the
  `Java` arm, so the `$` join has no test.

Unblocked by a fixture verb that registers a `JvmClass` under a
`JvmSource::JarEntry` without parsing anything.
`crates/platform-jvm/tests/class_lookup.rs` does this against the platform
directly; the acceptance fixture has no equivalent.

## An unresolvable type reference we can observe

Two claims the walk already satisfies fail their tests for want of an
observable rather than for want of behaviour:

- `package_type_boundary::a_type_prefix_leaves_no_way_back_to_a_package`
- `package_type_boundary::a_prefix_that_names_nothing_is_unresolvable`

Both assert `unresolvable-type` at the cursor, and no such diagnostic exists.
They carry `expected_failure`, which is honest but weak: each will go green when
the diagnostic ships rather than when resolution becomes correct, and each would
stay green if the walk broke.

Unblocked by either an `unresolvable-type` diagnostic, or a negative expectation
on the fixture. The negative expectation is much the cheaper of the two and
would tighten both tests the day it lands.

## Telling two equally named declarations apart

`one_name_declared_in_two_trees_is_ambiguous` asserts
`ambiguous_between("target", &["p.B", "p.B"])`. The two targets are distinct, but
`declaration_label` renders both as `p.B`, so the expectation cannot say which
pair it got, and it would still pass if both halves came from one file.

Unblocked by an expectation that names the declaring source, not only the label.

## Scopes

Nothing about what a source can see is observable here. The fixture drives
`Beans` with one flat file space, and `find_declarations_for` resolves with
`JvmScopeQuery::unscoped()`, so every file sees the whole lake. A claim like
"two modules each declare `p.B`, and each sees only its own" has no surface.

Unblocked by the engine picking a scope per source, plus a fixture verb to say
which container a file belongs to.
