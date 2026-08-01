# TODO

Everything we know is unfinished, in one place.

Every entry points at a file, so it can be checked by opening it. An entry that
points nowhere is a wish and does not belong here.

Four piles, because each one asks for something different: fix it, build it,
decide it, or wait for it.

## Wrong

Behavior that contradicts a specification we have read.

- **A local class is in scope above its own declarator.** JLS §6.3 gives it "the
  rest of the immediately enclosing block", and our scope span covers the whole
  block, so the position inside it is never consulted. The far end is right; a
  reference after the block does not reach it. The
  `resolution/tests/lexical.rs::a_local_type_is_wrongly_in_scope_before_it_is_declared`
  asserts the wrong answer on purpose, so fixing this turns that test red.

- **`Protected` grants access to everybody.** JLS §6.6.2 grants it to a subclass
  responsible for the implementation of the object, which needs a type hierarchy
  we do not have. The `crates/lang-java/src/accessibility.rs` answers `true`
  unconditionally and says so in a comment; a wrong `false` would squiggle
  correct code, so the choice is deliberate, but nothing tests either half.

## Missing

Not built yet. Nothing is wrong; there is just no code.

- **On-demand imports**, stage 4 of `resolve_type_name`. Type-import-on-demand
  (§7.5.2), static-import-on-demand (§7.5.4), and the implicit `java.lang.*`
  (§7.3). Six acceptance claims wait on this; see the header of the
  `tests/acceptance/tests/languages/java/type_resolution/type_imports_on_demand.rs`.

- **Module imports**, stage 5 (§7.5.5). Needs the lake to hold modules first.
  See the `module_imports.rs` header in the same directory.

- **Import suggestions**, stage 6. No JLS section; this is ours.

- **Inherited member types** (§§8.2 and 9.2). A member type of a superclass or a
  superinterface is in scope in the subclass, and we do not walk the hierarchy.
  Four claims wait on it, listed in the `scope_of_declarations.rs` and the
  `shadowing.rs` headers.

- **Qualified type references** (§6.5.5.2). The `resolve_type_name` returns
  `Unresolved` for anything that is not a simple name; the
  `resolve_canonical_name` walks a dotted name already, but only for imports.

- **A diagnostic model.** We emit two codes today, `inaccessible-member` and
  `cannot-find-symbol`. Import errors, access errors, unresolvable types and
  type parameter misuse have no code and no design. The pending tests we deleted
  had invented seven names for these; do not take them as a starting point,
  because nobody decided whether an import problem is four codes or one code
  with a reason.

- **JPMS.** A JDK goes into the lake as one image, so the whole runtime is
  visible to everything. See the `crates/engine/src/workspace.rs`; splitting it
  needs the lake to hold modules.

## Undecided

Cannot be built until we choose.

- **Does `depends_on` chain?** Direct edges only today, so `app -> lib -> core`
  gives `app` the sources of `lib` and not those of `core`. What a descriptor
  means is a workspace-layer question. Pinned by
  `workspace/tests/scopes.rs::a_dependency_of_a_dependency_is_out_of_reach`, so
  a change gets noticed.

- **Nothing validates `depends_on` ids.** An id naming no unit is dropped
  silently, and there is no cycle check. Dropping beats failing the whole
  import, but a typo should probably be visible somewhere.

- **Is a `JvmQualifiedName` dotted or internal?** It holds the JLS binary name
  with dots (`p.Outer$Inner`), while the JVMS spells it `p/Outer$Inner`. A
  reader of real class files will meet the other form, and one of the two has to
  convert.

- **What does `superclass: None` mean?** Today it is how `java.lang.Object`
  reads, and it is also how "we have not looked" would read. Those want to be
  different values.

- **Local and anonymous classes have no name we can build.** Their binary names
  take a digit sequence after the `$` (`Outer$1`), and the
  `JvmQualifiedName::nested` cannot spell that. The `enclosing: None` is also
  how a top-level type reads, so the model cannot say "this one is local".

- **Two containers claiming one name.** The `crates/platform-jvm/src/query.rs`
  filters and takes what comes; the order is a `HashMap`'s. This is shadowing
  and not ambiguity, so it wants ranking.

- **How are specification editions configured?** We read JLS 26 and hardcode it.
  A project on an older language level is a real case and we have no place to
  put the setting.

- **Which architectural properties deserve their own tests?** Revision
  snapshots, replacement, deletion and batching are exercised only through
  whatever else happens to touch them. Scope filtering got its own tests and
  found things, so the others might too.

- **When does a test cite a JLS section?** The `docs/TESTING.md` leaves this
  open. Most of ours cite one; what we have not said is what the citation buys
  us later.

## Can't test yet

The claim is clear and the observable does not exist. Nothing will tell us when
these become possible, which is why they are written down.

- **A compiled artifact in the lake.** The `Fixture::file` runs everything
  through the Java parser, so every source is a `JvmSource::SourceFile`. Nothing
  can put a class file, a jar entry or a runtime image in, so no acceptance test
  reaches the projection side of resolution: the `$` join in the `member_types`
  `Jvm` arm has no test at all, and neither does a source file beating a class
  file of the same binary name. Unblocked by a fixture verb that registers a
  `JvmClass` without parsing; the `crates/platform-jvm/tests/class_lookup.rs`
  does this against the platform already.

- **An unresolvable type reference we can observe.** Two claims in the
  `package_type_boundary.rs` want to say "this name reaches nothing", and the
  fixture can only ask what a name resolved to. Unblocked by an
  `unresolvable-type` diagnostic, or more cheaply by a negative expectation on
  the fixture.

- **Telling two equally named declarations apart.** The `declaration_label`
  renders both halves of an ambiguity as `p.B`, so
  `one_name_declared_in_two_trees_is_ambiguous` would still pass if both came
  from one file. This has already cost us: the label stops walking at a scope
  owned by a method, so a local class in `Test.m()` is spelled `p.Local` exactly
  like a top-level one, which is how the §6.3 bug above hid. Unblocked by an
  expectation that names the declaring source and not only the label.

- **A loser that does not exist yet.** Every "A shadows B" test where B is a
  stage we have not built passes because B contributes nothing, and would pass
  just as well with the precedence backwards. The fix is the other half: show
  that B wins on its own. The `resolution/tests/staging.rs` does this, and it is
  why the cases with both halves stayed and the rest are claims in file headers.
  Unblocked per stage, as each one lands.

- **What kind of type a declaration is.** The `projection.rs` maps each
  `JavaTypeKind` onto a `JvmKind` and nothing reads the result, so the mapping
  is write-only. The fixture cannot ask what a declaration *is*, only what a
  name reaches. Unblocked by the first real consumer of `kind`, and only then;
  an expectation added earlier would be an observable that exists for the test
  and for nothing else.

## Chores

- **Delete `expected_failure`.** 27 marks left, and 17 of them wait on
  diagnostic codes that have never existed. The mark fired once in the project's
  life. The prose in each file header is the honest replacement; the fixture
  loses the verb and the `reason` field.

- **The `parser.rs` and the `model.rs` still keep their tests inline.** 14 tests
  in 1399 lines and 4 in 737. Both will grow.

- **The `docs/TESTING.md` has three stale spots.** The `expected_failure`
  section goes with the mechanism; "ninety-five tests" is 38 now; and the Naming
  rule reads as if a test may never name a method, which is not what we meant.
