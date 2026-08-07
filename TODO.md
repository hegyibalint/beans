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
  we do not have. Two functions in `crates/lang-java/src/accessibility.rs` answer
  `true` unconditionally and say so in a comment: `is_accessible` for a
  declaration we parsed, and `is_compiled_type_accessible` for one we hold only a
  class file of, which puts every `protected` nested type of every jar and
  runtime image under the same choice. A wrong `false` would squiggle correct
  code, so it stays deliberate, and nothing tests it either way.

- **A dynamic constant as a bootstrap argument loses the class.** JVMS 26 §§4.4
  and 4.7.23 permit one; `cafebabe` 0.9 refuses the class outright, which
  `class_file/tests/compatibility.rs` pins. Reading a whole runtime image put a
  number on it: one class of JDK 26's 27,923, `jdk.jpackage`'s
  `PackageBuilder`. Ours to fix only by patching or replacing `cafebabe`.

- **A class we cannot find is accessible to everybody.** The
  `is_compiled_type_accessible` in `crates/lang-java/src/accessibility.rs` reads
  `None` as "access control does not apply", which JLS §8.1.1 says of a local or
  anonymous class. But the `class_access` in `crates/lang-java/src/query.rs`
  also answers `None` when it simply did not find the class: it throws the
  `model::Class` away at `view_of` and re-finds it by `(source, fqn)`, so any
  divergence between the two lookups — a revision skew, a `TypeTarget::Compiled`
  built somewhere else — turns into silent permission. §6.6.1 is not consulted
  at all in that case. The failure direction is the quiet one: a wrong `true`
  grants access and squiggles nothing.

  The fix removes the branch rather than patching it. `view_of` already holds
  the `&model::Class`, so carrying the level on `TypeTarget::Compiled` makes the
  question unaskable, and it also deletes the second full scan the entry in
  **Missing** below is about.

- **A nested class whose file says nothing about itself gets a confident wrong
  answer.** The `class_access` in `crates/platform-jvm/src/class_file.rs`
  explains at length why §4.7.6 must win over the header — javac widens
  `protected` to `ACC_PUBLIC` and drops `private` — and then falls back to that
  same header when no `InnerClasses` entry names this class. A producer that is
  not javac (ASM, an obfuscator, an older compiler) can omit the table, and then
  a `private` nested class reports `Package` and a `protected` one reports
  `Public`. Three lines below, the local and anonymous case answers `None`
  because it has no evidence; this branch has no evidence either and answers
  `Some`. No fixture covers a nested class without an `InnerClasses` entry.

- **`Thread$State` spelled as one identifier skips a check the dotted spelling
  makes.** `$` is a legal Java identifier character, so the parser hands
  `Thread$State` to stage 3 and stage 4 as a simple name, and
  `BinaryName::in_package` in `crates/lang-java/src/resolution.rs` glues it
  straight onto a package to reach the nested binary name. The
  `resolve_canonical_name` walk is careful here — it classifies `Thread` first
  and `propagate`s an inaccessible parent onto the member — but this path never
  sees the enclosing type, so a public nested type inside a package-private
  outer resolves with the outer unchecked. Narrow, and the only place where two
  spellings of one reference take different rules.

## Missing

Not built yet. Nothing is wrong; there is just no code.

- **The on-demand imports a user writes**, the rest of stage 4 of
  `resolve_type_name`. The implicit `java.lang.*` (§7.3) is built and is the
  whole of the stage; type-import-on-demand (§7.5.2) and static-import-on-demand
  (§7.5.4) reach the model as a `model::ImportKind` and are read by nobody. Both
  name a package *or* a type (§6.5.4), so both need the `resolve_canonical_name`
  walk to report which of the two its name ended in, and it discards the package
  half today.

- **Static imports** (§§7.5.3 and 7.5.4), single and on-demand alike. The
  `resolve_exact_imports` handles the single-type case only.

- **Module imports**, stage 5 (§7.5.5). Needs the lake to hold modules first.

- **Import suggestions**, stage 6. No JLS section; this is ours.

- **The members of a compiled type.** The `find_member` in the `resolution.rs`
  walks a `model::File`'s scopes, so a field or a method of a class file reaches
  nothing: `Instant.now()` has no answer even with the JDK in scope. Its type
  member sibling, the `member_types` `Compiled` arm, does the same job over binary
  names and shows the shape the rest would take. Until then `jvm::model::Field::access`
  and `jvm::model::Method::access` are decoded, pinned by
  `class_file/tests/declarations.rs`, and read by nobody.

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

- **Skipping a collapsed parse.** The `process` in
  `crates/lang-java/src/lib.rs` stores whatever came back and projects it, so a
  parse that recovered nothing replaces a file's model with an empty one and
  withdraws its classes from the lake. Measured on the engine: while `Foo.java`
  is mid-edit, its own analysis goes quiet, and an untouched `Bar.java` keeps a
  clean report while its `Foo f` drops from one target to none. Nothing squiggles
  today only because an unknown type has no diagnostic yet, which is the entry
  above; the day it does, every dependent file lights up whenever someone is
  typing.

  Decided: skip the write when a parse both errored and produced no top-level
  declaration, leaving the last good model live. Both conditions are needed —
  `has_error` alone is true for `class A {`, which recovers with a `MISSING }`
  and a complete model, and a file holding only a package declaration has no
  declaration and no error. Skipping is the whole implementation, because
  `RevisionedStorage::get` already answers with the newest version not newer
  than the revision asked for.

  What it asks of the parser: a collapsed parse is an ordinary outcome and has to
  be reported rather than asserted against. Tree-sitter's recovery is cost-based,
  and for some inputs the cheapest tree it finds throws the file structure away —
  `class A { void m() { int x = 1; y` parses to a bare `ERROR` root with no
  `program` above it, which the `debug_assert_eq!(root.kind(), "program")` in
  `crates/lang-java/src/parser.rs` currently treats as impossible. Nothing in a
  `model::File` can say a parse recovered nothing, and `has_error` lives only on
  the tree, so the judgement is the parser's to make and to hand back. `parse`
  returning `Option<model::File>` is the small answer; a status the model carries is
  the larger one, and it is what a consumer would need to know it is being served
  something stale.

  Accepted with it: the kept model's spans point into text that has changed, so
  navigation inside that one file can land wrong until it parses again, and a
  deleted class stays visible for as long. Stale is right for navigation and
  highlighting and questionable for diagnostics, which nothing distinguishes yet.

- **Repairing a collapsed parse instead of skipping it.** The better answer, and
  not the one we chose. A collapse needs unbalanced braces *and* something
  unfinished; a balanced buffer never collapses, whatever is half-typed in it.
  Appending the missing braces restores a full tree, measured: `class A { void
  m() { int x = 1; Str` has no scopes at all, and the same text with `}}` parses
  to a complete program. Tree-sitter cannot say where the brace belongs in
  exactly the cases that need it — a recovered parse reports one `MISSING "}"`
  per brace, and a collapsed one reports no `MISSING` at all and a single `ERROR`
  over the whole file — so the count has to be ours, which means a scanner that
  skips strings, char literals, comments and text blocks. An unterminated `"`
  breaks the count and stays a job for skipping. Would remove every cost the
  entry above accepts.

- **JPMS.** A JDK goes into the lake as one image, so the whole runtime is
  visible to everything. See the `crates/engine/src/workspace.rs`; splitting it
  needs the lake to hold modules. The module name is no longer the obstacle:
  `container/jimage.rs` reads it off every entry and glues it onto the front of
  the `jvm::model::Source::JimageEntry` path, so it is already there to be lifted into a
  field of its own once something can hold it.

- **A `jmod` is not a container.** `jvm::model::Source::JmodEntry` exists and nothing
  builds one; `container.rs` dispatches `.jar` and nothing else through
  `archive.rs`. JDK 25 and 26 ship no `jmods/` at all (JEP 493), so this only
  matters for an older JDK or a module path that names one directly.

- **`compact-cp` entries are named rather than read.** `jlink --compress=1`
  strips a class file's UTF-8 constant pool into the image string table and
  rewrites its descriptors, so undoing it means writing a constant pool back —
  before a class-file parser ever sees the bytes. `container/jimage.rs` reports
  the decompressor by name instead. `zip` is implemented; shipped JDKs use
  neither.

- **Nothing indexes the lake.** Every lookup is a linear scan: `all_classes` in
  `crates/platform-jvm/src/query.rs` walks every source × every class at the
  revision, and `classes_named` and `classes_in_package` both filter it.
  Resolution asks several times per name — stage 3, then stage 4's
  `java.lang.<Name>` probe, then `class_access` re-asking for a class the
  candidate already came from — and `analyze` runs the whole of
  `type_scope_diagnostics` on every `didChange`. With a JDK image in the lake
  that is ~30,000 classes per traversal, multiplied by the type references in
  the file, on every keystroke. Nothing is wrong with the answers; there is just
  no index to ask instead. A name-keyed and package-keyed map rebuilt per
  revision is the obvious shape, and the `class_access` half of it disappears
  anyway once `TypeTarget::Compiled` carries the level (see **Wrong**).

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

- **Is a `jvm::model::BinaryName` dotted or internal?** It holds the JLS binary name
  with dots (`p.Outer$Inner`), while the JVMS spells it `p/Outer$Inner`. A
  reader of real class files will meet the other form, and one of the two has to
  convert.

- **What does `superclass: None` mean?** Today it is how `java.lang.Object`
  reads, and it is also how "we have not looked" would read. Those want to be
  different values.

- **Local and anonymous classes have no name we can build.** Their binary names
  take a digit sequence after the `$` (`Outer$1`), and the
  `jvm::model::BinaryName::nested` cannot spell that. The `enclosing: None` is also
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

- **What does a unit with no JDK deserve to be told?** Since stage 4 reads the
  implicit `java.lang.*`, a unit that names no JDK in a workspace where another
  one does gets `type-outside-scope` on every `java.lang` name it uses —
  `String`, `Object`, `Integer`, `Exception`, each occurrence — because the
  image is in the lake but out of that unit's scope.
  `tests/acceptance/tests/engine/jdk.rs` pins the `String` case. Note the shape
  of it: if *no* unit names a JDK the same files are silent, because then
  nothing is in the lake to be out of scope. One misconfigured unit is a single
  fact about the unit, and we report it once per name in every file. A
  unit-level diagnostic would say it once, but that needs somewhere for a
  diagnostic that belongs to no file to live.

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
  `model::TypeKind` onto a `jvm::model::TypeKind` and nothing reads the result, so the mapping
  is write-only. The fixture cannot ask what a declaration *is*, only what a
  name reaches. Unblocked by the first real consumer of `kind`, and only then;
  an expectation added earlier would be an observable that exists for the test
  and for nothing else.

## Chores

- **A source file beating a class file of the same binary name is untested.**
  The workspace JAR acceptance test proves a compiled class is indexed and
  scoped, and `resolution/tests/compiled.rs` now walks the `$` join in the
  `member_types` `Compiled` arm, but nothing says which of the two wins when a
  project holds both.

- **The `parser.rs` and the `model.rs` still keep their tests inline.** 14 tests
  in 1399 lines and 4 in 737. Both will grow.

- **Acceptance is thin on purpose now, and we should watch it.** The Java type
  resolution capability is six tests, one per rule that reaches a user. If a bug
  ever gets out that all six missed, that is the signal one of them was not
  enough, and the answer is one more test rather than a return to the old tree.

- **No test imports a `java.*` type explicitly.** `grep -rn 'import java\.'
  tests/acceptance/ crates/lang-java/tests/` returns nothing, so
  `resolve_canonical_name` walking a package prefix against a real runtime image
  — `java` miss, `java.util` miss, `java.util.List` hit — is exercised only by
  `resolution/tests/compiled.rs`, over a synthetic `JarEntry` lake of hand-built
  `model::Class` values. Jimage entry paths, the module name glued onto
  `jvm::model::Source::JimageEntry`, and `BinaryName::in_package` could all break
  the multi-segment walk against real JDK data with nothing going red. One file
  in the `jdk.rs` fixture covers it.
