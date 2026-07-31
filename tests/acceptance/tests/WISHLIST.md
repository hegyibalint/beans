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

This has already cost us a real one. `declaration_label` stops walking when a
scope's owner is not a type, so a local class in `Test.m()` is spelled `p.Local`,
exactly like a top level `p.Local`. The acceptance test
`local_type_is_not_in_scope_before_its_declaration` asserted `p.Local` against a
file holding both and passed whichever one it reached, hiding the fact that
resolution reaches the local class from above its own declarator, against §6.3.
The unit test that replaced it, in `resolution/tests/lexical.rs`, records the
wrong answer on purpose so that fixing resolution turns it red.

Unblocked by an expectation that names the declaring source, not only the label.

## A loser that does not exist yet

Not a missing observable, but the opposite failure: an expectation that can be
stated, passes, and proves nothing. Every "A shadows B" test where B is a stage
we have not built passes because B contributes nothing, and it would pass just as
well with the precedence backwards.

There is no mark for this. `expected_failure` says a claim fails today, and these
do not fail; they are pending while looking settled, which is the worse of the
two. What each one needs is the other half: show that B would have won on its
own. `resolution/tests/staging.rs` does exactly that, and it is why the cases
that had both halves stayed and the ones that could not are listed as claims in
their file headers instead.

Unblocked per stage, as each one lands. Worth a fixture verb if it keeps
happening, though two `fixture()` calls in one test already express it.

## What kind of type a declaration is

`projection.rs` maps each `JavaTypeKind` onto a `JvmKind`, and nothing reads the
result. No resolution, diagnostic or query path has ever looked at `JvmClass::kind`,
so the mapping is write-only. The fixture matches: every expectation it offers is
about a diagnostic or about which declaration a name reaches, and none of them can
ask what that declaration *is*.

Blocked claim:

- Five files, one per declaration form, each coming back as its own kind: a class,
  an interface, an enum class (JLS §8.9), a record class (§8.10) and an annotation
  interface (§9.6). What this would catch is a form quietly collapsing onto
  `JvmKind::Class`, which is what dropping a variant from either enum would do.

`parser.rs::parses_each_named_type_kind` pins the five values on the Java side, so
the gap is the projection alone.

Unblocked by the first real consumer of `kind`, and only then by a fixture
expectation naming it. The order is the point: an expectation added before a
consumer would be an observable that exists for the test and for nothing else.
