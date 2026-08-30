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

- **`Protected` grants access to everybody, and now everybody can see it.** JLS
  §6.6.2 grants it to a subclass responsible for the implementation of the
  object; §6.6.2.1 adds that access from another package is permitted only
  through a reference whose type is the subclass. Two functions in
  `crates/lang-java/src/accessibility.rs` answer `true` unconditionally and say
  so in a comment: `is_accessible` for a declaration we parsed, and
  `is_compiled_accessible` for one we hold only a class file of.

  This used to cost nothing visible. It now costs two rows on *every* member
  popup in the language, because §8.1.4 puts `Object` above every type and
  `Object` declares `protected clone()` and `protected finalize()`. Measured
  against a real JDK: `widget.` offers twelve rows and two of them are those.
  Nothing else in the list is wrong — `notify`, `notifyAll` and `wait` are
  `public final` and belong there.

  The obstacle is gone, which is what changes the priority. `types_to_search`
  answers "is this type below that one" for both halves of the lake now, so
  §6.6.2 has the hierarchy it was waiting for; what is left is §6.6.2.1's rule
  about the type of the qualifying reference, and deciding what a caret inside a
  subclass should be offered on `this`. A wrong `false` would squiggle correct
  code, so it stays deliberate, and nothing tests it either way.

- **A dynamic constant as a bootstrap argument loses the class.** JVMS 26 §§4.4
  and 4.7.23 permit one; `cafebabe` 0.9 refuses the class outright, which
  `class_file/tests/compatibility.rs` pins. Reading a whole runtime image put a
  number on it: one class of JDK 26's 27,923, `jdk.jpackage`'s
  `PackageBuilder`. Ours to fix only by patching or replacing `cafebabe`.

- **A class we cannot find is accessible to everybody.** The
  `is_compiled_accessible` in `crates/lang-java/src/accessibility.rs` reads
  `None` as "access control does not apply", which JLS §8.1.1 says of a local or
  anonymous class. But the `class_access` in `crates/lang-java/src/query.rs`
  also answers `None` when it simply did not find the class: it throws the
  `model::Class` away at `view_of` and re-finds it by `(source, fqn)`, so any
  divergence between the two lookups — a revision skew, a `TypeTarget::Compiled`
  built somewhere else — turns into silent permission. §6.6.1 is not consulted
  at all in that case. The failure direction is the quiet one: a wrong `true`
  grants access and squiggles nothing.

  Now three lookups rather than two, because the hierarchy walk asks
  `compiled_class` for the same class again to read its members and its
  supertypes. They all go through one function at least, which is what makes the
  fix a rewrite of one place: `view_of` already holds the `&model::Class`, so
  carrying it — or at least the level — on `TypeTarget::Compiled` makes the
  question unaskable and deletes the repeated scans the entry in **Missing**
  below is about.

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

- **An anonymous class body is invisible, and `this` inside one points
  outward.** The `parse_expression` in `crates/lang-java/src/parser.rs` handles
  `object_creation_expression` by reading its `type` and its `arguments` and
  never its `class_body`, so nothing an anonymous class declares reaches the
  model. Measured: `new R() { int field; public void run(int arg) { int inAnon
  = 1; } }` inside `Outer.m` contributes the declarations `["Outer", "m"]` and
  nothing else — `field`, `arg` and `inAnon` do not exist. The same shape
  written as a local class yields all seven.

  The wrong answer is `this`. §15.9.5 makes it denote the anonymous instance;
  `File::enclosing_type_declaration` looks up the scope chain for a scope owned
  by a type declaration, finds none inside the anonymous body, and answers with
  the enclosing class. So a caret at `this.` in an anonymous class would be
  offered the *outer* class's members, confidently and with nothing to squiggle.

  The fix walks the body rather than patching the walk.
  `enclosing_type_declaration` is right for every construct we do model, and
  starts answering correctly on its own once an anonymous body owns a scope
  with a type declaration in it. What stands in the way is naming: §13.1 gives
  an anonymous class a digit sequence after the `$` that
  `jvm::model::BinaryName::nested` cannot spell, which is its own entry under
  **Undecided**.

- **A caret inside a comment or a string literal is offered the program's
  names.** §3.7 makes a comment's text not a token of the program and §3.10.5
  says the same of a string literal's body, so §6.5 has nothing to classify
  there and the answer is no list at all. Measured: with `class Widget { int
  own; }` in scope, a caret at `// see widget.‸ in prose` is offered `own`, and
  so is one at `String s = "widget.‸";`. The `Point::at` in
  `crates/lang-java/src/completion.rs` reads one character of lookbehind and
  `enclosing_scope` answers from a span that covers the comment along with the
  code around it, so neither half can tell.

  Latent until the LSP started firing on `.`. Ctrl-space did this all along and
  nobody met it, because nobody asks for completion inside a javadoc; a trigger
  character asks on every dot typed, javadoc included, and this file's own prose
  is full of them. Accepted deliberately rather than overlooked — the guard is
  worth more once there is something to build it on.

  Two ways to build it. A lexical scan of `contents` tracking comment, string,
  char and text-block state answers it in one pass, which next to the lake scan
  below costs nothing. The structural answer is the syntax tree, which would say
  what node an offset is in directly and replace the lookbehind too; that is the
  *Does lang-java keep the syntax tree?* entry under **Undecided**.

- **A dot with no name after it eats the rest of the enclosing class.**
  Tree-sitter reaches forward for the identifier a `.` needs, and whatever it
  reaches across stops existing. Measured on a class declaring `field`, `m`,
  `first` and `second`, with `this.` written inside `m`: the tree holds `field`
  and `m` and nothing else, both methods below being consumed as the
  invocation's name and argument list. §8.2 puts every member in scope
  throughout the body, so completing `this.` offers only what was declared
  above the caret, and go-to-definition on the rest fails while the dot is
  incomplete.

  How far it reaches depends on what precedes the dot. A bare identifier is
  benign: `a.` recovers into an `ERROR` holding `a` and the dot, and no sibling
  is lost. Anything the grammar already reads as an expression runs away —
  `this.` takes the rest of the class body, while `list.get(0).`, `items[0].`,
  `new Foo().` and `f(x, y).` each take the following member. Unbalanced
  brackets do not compound it: `foo(bar.`, `if (a.` and `new Foo(a.` all keep
  their enclosing method, and the receiver stays readable in every one.

  What it eats does not merely vanish, it is reparented. Measured in
  `examples/beans/Demo.java`: `this.` inside a method offers `Base`, `Marker`
  and `Widget` as member types of `Demo`, because the consumed run swallowed
  `Demo`'s own closing brace and the three top level siblings after it landed
  inside its body scope. So the list is both short and salted.

  Worse when nothing follows at all. With `this.` written in a class's *last*
  member there is no member left to eat, and the whole `class_declaration`
  collapses into an `ERROR` — no type declaration, no method, no scope, so
  completion has nothing to stand in rather than a short list. That is the
  collapsed parse the **Missing** entry below is about, reached by a trailing
  dot rather than an unbalanced brace, and it is why
  `completion/tests/members.rs::this_offers_the_members_above_the_caret_and_loses_those_below`
  has to declare a member after the caret's method.

  `super.` fails the other way round. It consumes nothing, and produces no
  member access node at all, so there is no receiver to read.

  Self-correcting, which is what keeps it out of sight: one more keystroke and
  `this.g` parses to a clean `field_access` with every member back, so only the
  first popup after the dot is short.

  The fix is the trick IntelliJ and rust-analyzer both use — insert a
  placeholder identifier where the name is missing, parse that, and record the
  name as missing rather than as the placeholder. Measured, `x;` restores every
  case above; the semicolon earns its place because `something.` followed by
  `int after = 1;` recovers the following statement only with it. What it costs
  is a second `model::File` for one source, and therefore a second
  `model::DeclarationId` space. The `resolve_receiver_class` in
  `crates/lang-java/src/resolution.rs` starts from the model it was handed, and
  every step after it — `find_member`, `type_of_member`, the hierarchy walk —
  re-fetches whatever `Query::model_of` returns for a source, so the two have to
  be the same model and nothing says they are.
  Deferred until a request-scoped model can be made visible to one query and to
  nobody else, which is the same mechanism an uncommitted batch import needs.

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

- **Import suggestions**, stage 6. No JLS section; this is ours, and it is the
  absence a user meets first: `ConcurrentHashMap` is on the classpath and in the
  lake, and §6.5.5.1 wants a simple type name to be *in scope*, which it is not
  without an import. So the row has to carry the import along with the name.

  Four things it needs. A prefix query over the whole lake, where today's is per
  package — and the one place the linear scans really bite, since it walks 28,000
  classes rather than a package's 145. An item that carries an edit;
  `CompletionItem` has `replace` and no insert. Somewhere to put that edit, which
  `model::Import` cannot answer: it carries the name's span, not the
  declaration's, so "after the last import" is not a question the model can be
  asked. And one name stops being one row — `List` is `java.util.List` *and*
  `java.awt.List` — which is the only piece that is shape rather than effort, and
  nothing built forecloses it, because an importable candidate does not shadow
  and appends rather than merges.

  Undecided with it: what to do when an importable name is already in scope. It
  cannot be imported without a conflict, so it is either dropped or offered with
  a fully-qualified insertion instead. IDEA does the latter.

- **A type position is offered variables and methods too.** The `Context` in
  `crates/lang-java/src/completion.rs` distinguishes a qualified caret from an
  unqualified one and nothing else, so `new ‸` and a type annotation get the
  whole list. §4.11 names the 17 contexts where a type is what is wanted.
  Harmless in the way a superset is, which is why it is still here. With it comes
  the one test never written, `namespaces.rs`: a type position offers no locals.
  The other half of that claim, that a dot offers nothing, is in `lexical.rs`.

- **Every name completion offers is resolved to decide whether to offer it.**
  One resolution per candidate per keystroke, which with a JDK in scope is the
  770 ms the index entry above is about. The shape was wanted before the speed,
  and an index is what replaces the scan rather than a cache around it.

- **Nothing ever drops a revision.** `Beans::process` bumps one per call and the
  LSP declares `TextDocumentSyncKind::FULL`, so an editor sends the whole
  document on every keystroke; `text_files`, `file_models` and the lake each
  `put` a full copy, and `RevisionedStorage` appends without ever removing. A
  long editing session therefore retains every version of every file's text and
  model, and there is no pruning, compaction or watermark anywhere in
  `crates/core/src/storage.rs` or `crates/engine/src/lib.rs`. What is missing is
  a floor revision — the oldest anyone can still ask about — below which
  versions can collapse. Storing the syntax tree, below, multiplies whatever
  this costs.

- **Inherited members, for a name that is not written after a dot.**
  `resolution/hierarchy.rs` walks §8.2 and §9.2 now and everything qualified
  goes through it, into the lake and back: `widget.inherited` completes and
  navigates, `text.toString()` reaches `java.lang.Object` through a real runtime
  image, hiding and overriding collapse onto the nearest declaration, and a
  diamond or a cycle terminates. What is not wired to it is the unqualified
  half — a bare `inherited` inside `Widget`'s own body.

  Half of that half already works, which is the confusing part. A call resolves,
  because `resolve_expression` sends `MethodCall` through the walk whether or
  not a receiver was written, and `navigation.rs::an_unqualified_call_reaches_an_inherited_method`
  pins it. Completion does not, because `methods_in_scope` reads one body scope,
  and neither does a bare name, because `variables_in_scope` walks the lexical
  chain and says of itself that it is always in-file.

  What it costs is the same thing the walk already cost the qualified side: a
  `model::DeclarationId` stops being enough on its own. `resolve_variable_name`
  returns bare ids to two callers that pair them with the *asking* file's
  source, and an inherited field is not in that file — `Member` is the shape it
  would have to grow into. Ordering needs a decision too: an inherited field
  sits at the depth of the body scope that inherited it, so it has to lose to a
  local and beat an enclosing class's field, and `InScopeVariable::depth` is
  where that would be said.

- **A bridge method is offered as if a user had written it.** JVMS §4.6 marks a
  compiler-generated method `ACC_SYNTHETIC` or `ACC_BRIDGE`, and
  `crates/platform-jvm/src/class_file.rs` decodes neither, so both reach the
  lake as ordinary methods. Measured against a real JDK: `String` offers
  `compareTo(Object)` beside `compareTo(String)`, the first being the bridge
  javac emits for `Comparable`. §8.4.2 makes them different signatures, so the
  dedup in `completion.rs` keeps both, correctly — the row should not be there
  at all. Two flags on `jvm::model::Method` and one filter.

- **A member type is offered through inheritance and cannot be resolved through
  it.** §8.5 inherits a member type, and `completion.rs`'s `members` walks the
  `Type` namespace along with the other two, so `Widget.Nested` is offered when
  `Nested` is declared in `Base` — pinned by
  `completion/tests/inheritance.rs::a_member_type_of_a_superclass_is_offered`.
  The `member_types` in `resolution/types.rs` reads one body scope, so the name
  it offers resolves to nothing.

  Not an oversight. That function is a stage of `resolve_type_name`, and the
  walk resolves each supertype by calling `resolve_type_name` — wiring one into
  the other is a cycle, and it needs a guard or a separate entry point before it
  is safe.

- **A `static` member is offered through an instance receiver with nothing to
  mark it.** §15.12.1 permits `text.format(...)` — a static method reached
  through an *ExpressionName* — so the row is not wrong, and against a real JDK
  `String.` carries eleven `of`-style rows a reader did not ask for. Neither
  `jvm::model::Method` nor `model::MethodDeclaration` records `static` at all,
  so there is nothing to rank or grey out by. Every IDE de-emphasises these.

- **A record's and an enum's own implicit members.** §8.10.3 gives a record an
  accessor per component plus `equals`, `hashCode` and `toString`; §8.9.3 gives
  an enum a `public static final` field per constant plus `values()` and
  `valueOf(String)`. Neither is inherited, so no amount of hierarchy walking
  produces them: they are the type's own members that the source never spells.
  `record Point(int i, int j)` therefore offers no `i()` and no `j()`.

  A third category, and the model has no word for it. Everything in
  `model::File` came from a token the parser read. These would have to be
  synthesized — in the parser, where the record components are in hand, or in
  projection, where the lake would carry them for a compiled reader too.

- **The walk is a traversal per keystroke.** `types_to_search` resolves every
  supertype name from scratch on every call, and `members` runs it once per
  namespace — three walks per popup, each a `resolve_type_name` or a
  `classes_named` per hop. Measured in release against a real JDK: 2.8 ms for a
  local class, 9.5 ms for `List`, 13.4 ms for `String`, whose hierarchy is
  `CharSequence`, `Comparable`, `Serializable`, `Constable`,
  `ConstantDesc` and `Object`. Cheap next to the 770 ms an *unqualified* caret
  costs, because that one resolves all 145 `java.lang` names and this resolves a
  handful. Do not cache a hierarchy on top of a linear scan; index the lake
  (below) and the walk is free.

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

  Completion put a number on it, being the thing that asks the most: one
  `classes_named` is a 2.75 ms pass over ~28,000 classes, and one keystroke in a
  file with a JDK in scope costs 770 ms. That is 145 top-level `java.lang`
  names, each scanned for twice — once to resolve and once for `class_access`.
  Ten runs back to back gave 776 ms first and 776 ms last, so there is no
  warm-up anyone is waiting on. Kept on purpose: the scans are what show where
  an index would pay, and replacing them before the shape is settled would be
  guessing.

- **A lambda body is invisible.** The `parse_expression` in
  `crates/lang-java/src/parser.rs` has no `lambda_expression` arm, so one falls
  to `_ => return None` and neither its parameters nor its statements reach the
  model. Measured: `run(p -> { int inLambda = 1; })` inside `Outer.m`
  contributes the declarations `["Outer", "m"]`, so neither `p` nor `inLambda`
  is in scope at a caret inside the lambda.

  Absent rather than wrong, which is what separates it from the anonymous class
  entry above: `this` is accidentally right here. §15.27.2 gives a lambda body
  the same `this` as its surrounding context, and `enclosing_type_declaration`
  looks for a scope owned by a *type* declaration, which a lambda never is. It
  stays right once lambdas are parsed, because the scope a lambda introduces
  still has no type owner.

  What it needs: the grammar's `lambda_expression` carries `parameters` and
  `body`, the body being a block or a single expression (§15.27). A parameter
  may be written without a type (`p ->`), which `ParameterDeclaration.ty` being
  `Option` already allows; saying what `p` *is* needs the target functional
  interface type (§15.27.3), and there is nothing to ask that of yet.

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

- **Does lang-java keep the syntax tree?** The `Parser::parse` in
  `crates/lang-java/src/parser.rs` drops the tree-sitter `Tree` as soon as
  `parse_program` has walked it, so the only thing that outlives a parse is the
  `model::File`. Keeping it means keeping the text too: a `Tree` holds offsets
  and nothing else, which is why every function in that file takes both a `Node`
  and an `&str`.

  What it would unlock: structural edits, which is the real motivation —
  Roslyn, JDT and rust-analyzer all compute edits against a full-fidelity tree
  and emit text edits at the boundary, while a `model::File` is a semantic arena
  with no syntax in it. Also semantic highlighting, and a grammar-based answer to
  what a caret sits in, where `completion.rs` reads one character of lookbehind.
  Incremental parsing is on the list and blocked regardless: the LSP declares
  FULL sync, so there are no incremental edits to feed it.

  What makes it safe to defer: incremental parsing was measured to produce
  byte-identical trees, so a stored tree buys speed and edits and never
  correctness. Nothing decided later is foreclosed by not storing one now.
  `Tree` is `Clone`, `Send` and `Sync` in tree-sitter 0.25, so
  `RevisionedStorage<Source, Tree>` is mechanically fine; what it needs first is
  the floor revision above.

- **How are specification editions configured?** We read JLS 26 and hardcode it.
  A project on an older language level is a real case and we have no place to
  put the setting.

- **Is a runtime image read eagerly?** It is today: 27,000 classes and 116 MiB
  parsed and projected before a line of user code is, which takes about four
  seconds of the debug suite. The index is 1% of the file and already knows
  every name, and the perfect hash `container/jimage/index.rs` reads past
  exists to answer one of them at a time. What blocks the change is storage:
  nothing says what `jvm::Platform` holds for a name it has not projected yet.
  Measured at the engine surface rather than in the suite: `open_workspace` on
  `examples/beans` with `jdk_home` set takes 4.67 s before it answers anything.

  This is the one cost an index does not touch. Indexing the lake makes a lookup
  cheap and does nothing about filling it, because 28,000 class files are parsed
  either way. So the question is *when* rather than *how fast*, and it has two
  answers: read less, or serve while reading.

  Serving while reading needs one thing `Beans` does not do. `RevisionedStorage`
  already answers at whatever revision it is asked for, so serving an older world
  costs nothing. What fuses them is `Beans::process`, which bumps the revision
  and publishes it in the same act, leaving no revision that is written and not
  yet read. Splitting those would let a JDK load into the next revision while
  queries keep answering at the current one — the same caret with no JDK in the
  lake answers in under 0.1 ms, against 769 ms with one.

  Reading a JDK must not be what decides whether a caret gets an answer. Blocking
  is the current behaviour rather than a requirement, and nothing above it asked
  for it.

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

- **`crates/core` depends on `tree-sitter` and never uses it.** Nothing under
  `crates/core/src` names it. The dependency belongs to whoever parses, which is
  `lang-java`, and it already declares its own.

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
