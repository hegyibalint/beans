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

- **A compiled class has no access flags.** JLS §6.6.1 gates every type by its
  access modifier, and §7.3 imports only the `public` types of `java.lang`, but a
  `JvmClass` carries nothing to check: `class_file.rs` reads `access_flags` for
  the kind and drops the rest. So `type_target_is_accessible` in
  `crates/lang-java/src/resolution.rs` answers `true` for everything that is not
  a Java source, and `java.lang.Shutdown` resolves by simple name exactly as
  `java.lang.String` does. Stage 4 made it reachable without an import; a
  single-type import of a package-private class in a jar has always resolved.
  Nothing tests either half.

- **A dynamic constant as a bootstrap argument loses the class.** JVMS 26 §§4.4
  and 4.7.23 permit one; `cafebabe` 0.9 refuses the class outright, which
  `class_file/tests/compatibility.rs` pins. Reading a whole runtime image put a
  number on it: one class of JDK 26's 27,923, `jdk.jpackage`'s
  `PackageBuilder`. Ours to fix only by patching or replacing `cafebabe`.

## Missing

Not built yet. Nothing is wrong; there is just no code.

- **The on-demand imports a user writes**, the rest of stage 4 of
  `resolve_type_name`. The implicit `java.lang.*` (§7.3) is built and is the
  whole of the stage; type-import-on-demand (§7.5.2) and static-import-on-demand
  (§7.5.4) reach the model as a `JavaImportKind` and are read by nobody. Both
  name a package *or* a type (§6.5.4), so both need the `resolve_canonical_name`
  walk to report which of the two its name ended in, and it discards the package
  half today.

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
  needs the lake to hold modules. The module name is no longer the obstacle:
  `container/jimage.rs` reads it off every entry and glues it onto the front of
  the `JvmSource::JimageEntry` path, so it is already there to be lifted into a
  field of its own once something can hold it.

- **A `jmod` is not a container.** `JvmSource::JmodEntry` exists and nothing
  builds one; `container.rs` dispatches `.jar` and nothing else through
  `archive.rs`. JDK 25 and 26 ship no `jmods/` at all (JEP 493), so this only
  matters for an older JDK or a module path that names one directly.

- **`compact-cp` entries are named rather than read.** `jlink --compress=1`
  strips a class file's UTF-8 constant pool into the image string table and
  rewrites its descriptors, so undoing it means writing a constant pool back —
  before a class-file parser ever sees the bytes. `container/jimage.rs` reports
  the decompressor by name instead. `zip` is implemented; shipped JDKs use
  neither.

## Undecided

Cannot be built until we choose.

- **Does `depends_on` chain?** Direct edges only today, so `app -> lib -> core`
  gives `app` the sources of `lib` and not those of `core`. What a descriptor
  means is a workspace-layer question. Pinned by
  `workspace/tests/scopes.rs::a_dependency_of_a_dependency_is_out_of_reach`, so
  a change gets noticed.

- **A pushed workspace reads nothing.** The `set_workspace` records the
  structure without touching disk, so its scopes name jars and runtime images
  nothing ever opened; only `open_workspace` reads them. Fine while a
  descriptor on disk is the only real caller, and wrong the moment an editor or
  a build-tool import hands a workspace over instead. Either it reads, or the
  caller needs a second call, and neither has been chosen.

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

- **Is a runtime image read eagerly?** It is today: 27,000 classes and 116 MiB
  parsed and projected before a line of user code is, which takes about four
  seconds of the debug suite. The index is 1% of the file and already knows
  every name, and the perfect hash `container/jimage/index.rs` reads past
  exists to answer one of them at a time. What blocks the change is storage:
  nothing says what `PlatformJvm` holds for a name it has not projected yet.

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
  resolution capability is six tests, one per rule that reaches a user. If a bug
  ever gets out that all six missed, that is the signal one of them was not
  enough, and the answer is one more test rather than a return to the old tree.
