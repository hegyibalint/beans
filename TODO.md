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
  (§7.3). The stage is written out in the `resolution.rs` as a comment and
  returns nothing.

- **Static imports** (§§7.5.3 and 7.5.4), single and on-demand alike. The
  `resolve_exact_imports` handles the single-type case only.

- **Module imports**, stage 5 (§7.5.5). Needs the lake to hold modules first.

- **Import suggestions**, stage 6. No JLS section; this is ours.

- **Inherited member types** (§§8.2 and 9.2). A member type of a superclass or a
  superinterface is in scope in the subclass, and we do not walk the hierarchy.
  The same hierarchy is what `Protected` needs above.

- **Qualified type references** (§6.5.5.2). The `resolve_type_name` returns
  `Unresolved` for anything that is not a simple name; the
  `resolve_canonical_name` walks a dotted name already, but only for imports.

- **A diagnostic model.** We emit `type-outside-scope`,
  `inaccessible-member`, and `cannot-find-symbol`. Import errors, genuinely
  unknown types, ambiguous types, inaccessible types, and type parameter misuse
  still have no complete design. The pending tests we deleted had invented
  seven names for these; do not take them as a starting point, because nobody
  decided whether an import problem is four codes or one code with a reason.

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

- **Two containers claiming one name.** JVM discovery preserves both, and Java
  currently treats two in-scope declarations as ambiguity. A class path would
  rank containers instead; its current order is a `HashMap`'s.

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
  that B wins on its own. The `resolution/tests/staging.rs` does this, which is
  why only the pairs with both halves survived the sweep. Unblocked per stage,
  as each one lands; until then §6.4.1 is written down here and nowhere else.

- **What kind of type a declaration is.** The `projection.rs` maps each
  `JavaTypeKind` onto a `JvmKind` and nothing reads the result, so the mapping
  is write-only. The fixture cannot ask what a declaration *is*, only what a
  name reaches. Unblocked by the first real consumer of `kind`, and only then;
  an expectation added earlier would be an observable that exists for the test
  and for nothing else.

## Chores

- **Compiled resolution coverage stops at top-level scope.** The workspace JAR
  acceptance test proves a compiled class is indexed and scoped, but the `$`
  join in the `member_types` `Jvm` arm has no test at all, and neither does a
  source file beating a class file of the same binary name.

- **The `parser.rs` and the `model.rs` still keep their tests inline.** 14 tests
  in 1399 lines and 4 in 737. Both will grow.

- **Acceptance is thin on purpose now, and we should watch it.** The Java type
  resolution capability is four tests, one per rule that reaches a user. If a
  bug ever gets out that all four missed, that is the signal one of them was not
  enough, and the answer is a fifth test rather than a return to the old tree.
