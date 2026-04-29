# ADR-0021: Preserve the tree-sitter walker; rewrite the layers above it

## Status

Accepted

## Context

The current `beans-lang-java` implementation is built on tree-sitter.
Its core file, `beans-lang-java/src/parser.rs`, is roughly 1000 lines
of code that walks the tree-sitter parse tree, extracts symbols, and
emits a `Vec<Symbol>`. The model it produces — `Symbol`, `SymbolKind`,
`SymbolTable`, the `Language` trait — is being replaced by the new
node/registry/graph engine architecture (ADR-0006 and the surrounding
ADRs). The parser sits on top of that model: it constructs the model
types from tree-sitter nodes.

The question for the migration is: what do we keep, and what do we
rewrite?

The walker code is not generic. It knows that a `class_declaration`
in tree-sitter-java has a `name` field that is an `identifier`, that
modifier flags appear in a `modifiers` child node, that anonymous
classes show up under `object_creation_expression`, that record
components live in `formal_parameters` of a `record_declaration`,
and dozens of similar grammar quirks. That is not glamorous code,
but it is *correct* code, and getting it correct again means walking
the same grammar with the same edge cases. There is no leverage to
be had from rewriting it.

What does need to change is everything the walker emits. The output
type is `Vec<Symbol>`, where `Symbol` is the old monolithic struct
with `kind`, `parent`, `children`, `relations`, `signature`, and so
on packed into one record. The new model has typed node payloads,
graph edges (hard and dynamic links), and registries. The walker
should emit those directly, not produce `Symbol` and have something
else convert it.

## Decision

The migration preserves the tree-sitter integration and rewrites
the layers above it.

**Preserved:**

- The `tree_sitter::Parser` setup and tree construction.
- The recursive walk over tree-sitter nodes
  (`extract_symbol`, `extract_class_like`, `extract_method`, the
  whole family of `extract_*` functions in `parser.rs`).
- The handling of grammar-specific quirks: package extraction,
  modifier parsing, generics, record components, anonymous classes,
  enum constants with bodies, lambda parameter inference, and the
  rest.
- The position/span extraction (mapping tree-sitter byte ranges to
  source positions).
- The Java type-reference parser in `beans-lang-java/src/types.rs`.

**Rewritten:**

- The output. The walker no longer emits `Symbol`/`SymbolTable`
  entries. It emits new node payloads directly, with hard links to
  parents and children and dynamic links to references resolved
  later through registries.
- The `Language` trait is gone. Its place is taken by feature-gated
  language modules in `beans-core` (ADR-0019), each registering its
  parser entry point and its rule set with the engine.
- Symbol IDs (the `SymbolId` arena index) are replaced with graph
  node IDs allocated by the engine.
- The `signature: Option<Signature>` field is replaced by typed
  payload variants — a method node carries its return type and
  parameters as fields of its payload, not as an opaque
  `Signature`.
- Resolution. The old code resolved cross-references by FQN lookup
  in the symbol table; the new code goes through registries with
  query-and-fallback semantics.

The mechanical shape of the change: the `extract_*` functions keep
their tree-sitter signatures (`fn extract_class_like(ctx, node)`)
but the body that builds a `Symbol` is replaced by code that
allocates a node, sets typed payload fields, and records hard/dynamic
links on the context.

## Consequences

**Positive.**

- The grammar-quirk knowledge in the walker is preserved. We do
  not re-litigate edge cases like "where do annotations on a record
  component appear in the tree-sitter-java grammar."
- The migration is bounded in scope. We are not rewriting tree-sitter
  integration, only the model it feeds.
- The walker can be migrated incrementally: one `extract_*`
  function at a time can switch from emitting `Symbol` to emitting
  node payloads, with a temporary adapter to keep the rest working.
  This makes the change reviewable in slices rather than as one
  10,000-line PR.
- New languages added later (Kotlin, Scala, Groovy, Clojure) start
  from the same pattern: tree-sitter walker + typed payload emission.
  The Java module is the template.

**Negative.**

- The walker is currently coupled to the old model in subtle ways
  (e.g., it maintains an `enclosing_stack` of `(usize, String)` for
  parent linking, where `usize` is a `SymbolId`-equivalent). We have
  to replace those internals while leaving the structure intact.
  This is delicate work and the diff will look messy until it
  settles.
- During the migration window, the walker emits a hybrid output —
  some functions produce new node payloads, others still produce
  `Symbol`. A temporary adapter layer is needed to bridge them.
  That adapter is throwaway code; it must be removed when the last
  function is converted. We commit to deleting it (not letting it
  rot as a "compatibility shim").
- We are betting that tree-sitter remains a viable parsing layer.
  If we ever decided to switch to a hand-written or different
  parser, we would have to rewrite the walker after all. That bet
  is fine — tree-sitter has served Java well, and the alternatives
  (JavaParser, javac internals, Eclipse JDT) all carry larger costs
  and JVM dependencies.

## Alternatives considered

**Wholesale rewrite: throw out everything, start fresh.** Delete
`beans-lang-java` and write the new module against tree-sitter from
scratch. Rejected because the grammar-quirk handling is a body of
work that does not become wrong when the model around it changes.
Rewriting it from scratch buys nothing except a cleaner-looking
diff, at the cost of reintroducing every bug we fixed the first
time. The new model is the interesting change; the walker is not.

**Incremental port: build the new module in parallel and migrate
piece-by-piece, leaving the old one running alongside.** Stand up
a new `beans-lang-java-v2` module, port one symbol kind at a time,
and switch consumers over gradually. Rejected because the model
change is end-to-end — `Symbol` and the new node types do not
coexist meaningfully. A use site either resolves through the old
symbol table or through registries, not both. Running them in
parallel would mean every cross-reference has to know which world
its target lives in, which is more complexity than just doing the
migration. The walker-preserves, model-rewrites approach gives us
the same incremental property (function-by-function in the walker)
without the parallel-modules tax.

**Keep `Symbol` as an internal intermediate, convert to new node
types at the boundary.** The walker emits `Symbol` as before; a
conversion layer translates `Symbol` to node payloads. Rejected
because it preserves the old model as a fossil — every new feature
has to either thread through `Symbol` (paying the conversion cost)
or bypass it (defeating the abstraction). The conversion layer
becomes load-bearing and never goes away. We would rather pay the
migration cost once than carry two models forever.

**Replace tree-sitter as well.** Use a hand-written Java parser or
bind to javac. Rejected as out of scope: this ADR is about the
migration of the model layer, not the parsing layer. Tree-sitter
is fine. Revisiting that choice is a separate decision (and a much
larger one).
