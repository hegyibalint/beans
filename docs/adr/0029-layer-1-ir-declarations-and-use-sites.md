# ADR-0029: The layer-1 IR contains declarations and use sites, partitioned by modifiability

## Status

Accepted.

## Context

Layer 1 today is a *declaration index*. The graph contains class, method,
field, constructor, parameter, enum-constant, and package nodes — every payload
variant carries `JavaDeclHeader` and registers in `java_symbols`. The
tree-sitter walker extracts these declarations, captures their type signatures
as inert `TypeRef` *values*, and discards the parse tree. There are no nodes
for *uses* of any name: no type-references-as-graph-nodes, no method-calls,
no field-accesses, no identifier-reads.

This shape is enough to support `documentSymbol`, hover on declarations, and
go-to-definition by simple-name lookup. It is **not enough** for the work the
project's value proposition requires:

1. **Project-wide diagnostics.** A typo in `foo.bar.Service` should produce a
   warning at every call site of `Service` across the workspace, not just at
   the declaration. There is no graph object to attach that warning to today,
   because the call sites are not nodes.
2. **Cross-file refactor.** Renaming `Service` should rewrite every use site.
   With no use-site nodes, the only options are re-walking each file's parse
   tree at refactor time (correct but slow, and at odds with the registry
   architecture) or storing references in a parallel index (clangd-shaped,
   but at that point the graph and the index are duplicating ownership of
   "what references what").
3. **Cross-language reference flow.** The `FallbackSubscription<P, F>` shape
   from ADR-0008 was specified for cross-language use sites — a Java caller
   resolving a method that may be defined in Kotlin source or in a JAR. With
   no use-site nodes, no `FallbackSubscription` is ever constructed; the
   architecture's central cross-language mechanism has no consumers.

The first three rules from the diagnostic-rule backlog (#015) all collapse
to the same missing piece. `unused-import`, `reference-not-found`, and
`inaccessible-member` are each "look at use sites in this file and decide
something." They cannot be implemented against an IR that contains no use
sites.

A second axis surfaces alongside this: **not every node in the graph is
modifiable.** A `Service` defined in workspace source can be renamed, marked
deprecated, or have its members reorganized. A `java.util.HashMap` loaded from
the JDK's `java.base` JMOD cannot. Both are providers in the same registries;
they're indistinguishable to the registry layer. Refactor and quick-fix
operations need to filter out the second category, and diagnostics need to
report problems at modifiable positions. Without an explicit axis, every
consumer rolls its own "is this from a JAR?" check.

Five real choices were on the table for representing use sites — see
*Alternatives considered* below. The decision picks one and pairs it with
a modifiability axis that makes the asymmetric scope rule (which uses are
nodes, which aren't) follow from one principle instead of three.

## Decision

The layer-1 IR contains **two kinds of typed node**: declarations and use
sites. Both have precise spans pointing at a single identifier in source.
Both are partitioned by **modifiability**, an axis of the IR rooted at each
file/dependency/JMOD node.

### Use sites are first-class nodes

For every Java payload variant `JavaXxxNode` that represents a declaration,
there is a sibling family of variants representing references to declarations:

```
JavaTypeUseNode        — a named type in source position (supertype, field
                         type, parameter type, return type, throws,
                         type-bound, local-var type)
JavaMethodCallNode     — a call expression's method-name token
JavaFieldAccessNode    — a field-read or field-write's field-name token
JavaIdentifierReadNode — a bare identifier whose target may live in another
                         class (the receiver of a method-call, an
                         imported static name)
JavaConstructorCallNode — a `new T(...)` site
JavaAnnotationUseNode  — `@Deprecated` and friends
```

Each carries a `JavaUseHeader { name, location, candidate_fqns }`. The
`location` spans **only the identifier text**, not the surrounding
expression: `Repository<User>` emits two flat `JavaTypeUseNode`s — one for
`Repository`, one for `User` — each with its identifier-only span. This
invariant is load-bearing: it is what makes mechanical rename possible. A
multi-identifier expression's structural composition (e.g., which is the
type-arg, which is the outer raw type) is recoverable from the `TypeRef`
value already on the enclosing declaration; the IR nodes carry only the
information rename and find-references need.

Use sites are hard-linked under the declaration that contains them. Walking
a file's roots' subtree yields every use site in the file, filtered by
payload kind.

`JavaUseHeader::candidate_fqns` is a parser-time best-effort resolution:
imports + same-package + java.lang + same-file types, in priority order.
Resolution at use time is "first FQN that hits `java_symbols`." No
`FallbackSubscription` is constructed in slice 1; the use site is a passive
data carrier. Subscriptions are added when the layer-2 cache design needs
precise invalidation (per ADR-0027's "wait for the real driver").

### Modifiability is an axis

Every node has an origin, expressed by walking parent hard-links to a root:

- **Modifiable** roots (`file://<workspace-path>`): workspace source files.
  Their descendants — declarations, use sites, JVM projections — are all
  modifiable.
- **Read-only** roots (`dependency://<coord>`, `jmod://<module>`): JAR
  members, JMOD members, compiled `.class` files. Their descendants —
  declarations and JVM projections only; never use sites — are all
  read-only.

Use sites appear **only under modifiable roots.** A read-only source has
already been compiled; its references are recorded as JVM bytecode
mnemonics, not as source identifiers we could squiggle or rename. There is
no consumer for "list every callsite in `rt.jar`," and constructing those
nodes would balloon the graph for no benefit.

Consumers query modifiability via `NodeData::origin()`-shaped helpers
(walking to root). The implications:

- **Diagnostics** report only at modifiable positions. A diagnostic *about*
  a read-only target — e.g., calling a deprecated JDK method — fires at the
  caller's modifiable use site, not at the JDK declaration.
- **Refactor scope** is modifiable nodes only. Renaming `Service` rewrites
  modifiable declarations and modifiable use sites; JAR declarations of the
  same FQN are skipped.
- **Resolution** prefers modifiable providers when both exist for the same
  key. The source-vs-JAR collision (e.g., a workspace shadowing a library
  class) resolves to the source by default. ADR-0008's
  `FallbackSubscription<P, F>` already encodes this shape — primary registry
  is per-language and source-only; fallback registry is JVM-wide and
  potentially read-only.
- **Code actions** never offer fixes on unmodifiable nodes (generalizes
  ADR-0028's "no actions on unreconciled cached entries" — the same safety
  rule, broader scope).

### Source emission rules

The two-mode emission is now a consequence of the IR shape, not an
optimization:

- **Mutable source parsing** (`.java`, `.kt`, `.scala`, `.groovy`, `.clj`):
  emits the full IR — declarations, use sites, JVM projections.
- **Read-only source parsing** (`.jar`, `.jmod`, `.class`, decompiled
  signatures): emits declarations and their JVM projections only. No
  use-site nodes; their would-be parents are unmodifiable, so their data
  has no consumer.

### Slice-1 scope

The first implementation slice introduces:

- `JavaTypeUseNode` only. The four other use-site variants are deferred
  until rules that need them land.
- Walker emission for type uses **in declaration headers** only: supertype,
  implements, field type, parameter type, return type, throws, type-bound.
  Type uses inside method/constructor bodies (local-variable types,
  generic-method type arguments at call sites) are deferred to the body
  slice that lands alongside `JavaMethodCallNode` and friends.
- `Location` added to `Import` so the unused-import diagnostic squiggles
  the import statement.
- `has_body: bool` added to `JavaMethodNode` so the
  abstract-method-with-body rule can fire on a structural fact already in
  the parse tree.
- The first two diagnostic rules — `abstract-method-with-body` (no use
  sites needed) and `unused-import` (uses `JavaTypeUseNode`).

The modifiability axis is not yet *encoded* on nodes — workspace source is
the only origin in the codebase today. The slot is reserved: when JAR
loading lands (backlog #012), file-root nodes (`file://`, `dependency://`)
will be introduced as the carriers of the origin tag, and existing helpers
will start consulting it.

## Consequences

**Positive.**

- Project-wide diagnostics become expressible. A use site is a node; its
  `Location` is the squiggle position; its `candidate_fqns` is the
  resolution input. Cross-file invalidation rides existing registry
  watches.
- Refactor primitives become expressible. Find-references on declaration
  X is "list every node subscribed to X's registry key (or whose
  `candidate_fqns` contain X's FQN)." Rename is "edit each use site's
  `Location.range`."
- The `FallbackSubscription<P, F>` mechanism finally has consumers. Each
  use site that targets a JVM-eligible kind constructs a subscription
  whose primary is per-language and whose fallback is JVM-wide.
- Read-only sources stay cheap. Parsing a 50 MB JAR produces decl nodes
  proportional to its public API, not to its caller graph. The graph
  doesn't pay storage for use sites that can't be edited or diagnosed.
- The decl/use partition is span-faithful by construction. Rename, extract,
  inline-variable, and change-signature compose against
  identifier-precise spans without secondary parse work.
- The asymmetry "intra-method local references stay out of the graph"
  follows from one principle (modifiability + cross-file scope) instead of
  being three separate calls. Local-variable reads, parameter reads, and
  this-field reads are all answered by per-file source walks at request
  time, with no graph node and no registry entry.

**Negative.**

- The walker grows. Every declaration-header position that today emits a
  `TypeRef` value now also emits a sibling `JavaTypeUseNode` with span
  extraction descending to the rightmost identifier (for
  `scoped_type_identifier`). Tested behavior: span of the use site is
  exactly the identifier text, no qualifier prefix. Easy to get wrong.
- Node count per file roughly triples for declaration-heavy files. A
  service class with 30 fields and 50 methods (each with parameters and
  return types) jumps from ~80 nodes today to ~250. Memory budget is fine
  at this scale; the body-slice expansion is where it gets tight (bodies
  carry hundreds of cross-class references each).
- Modifiability is a query, not a field. Per-node `is_modifiable()` walks
  to root every time. Negligible at slice 1, but a per-file or per-rule
  cache may be warranted once rules iterate the workspace. Optimization
  deferred until profiling shows it.
- Two ways to represent a name in source. A method's signature carries the
  parameter type both as a `TypeRef` value (on the parameter payload) and
  as a `JavaTypeUseNode` (under the parameter's parent declaration). Two
  separate places to keep in sync. The walker is the only writer of both,
  so divergence is bounded; a parser test pins the equivalence.

**Neutral.**

- Body-slice rules (`reference-not-found`, `inaccessible-member`,
  `inherited-member`) are unblocked but not yet implemented. They land
  rule-by-rule, each adding the use-site variants it needs.
- The serialization story (ADR-0028) is unaffected. Use-site nodes serialize
  the same way declarations do — payload is plain data, registry entries
  are runtime-only and rebuilt on snapshot load. The cached *artifact*
  (`Vec<Diagnostic>` per file) is a separate concern.
- Each consumer-defined query against use sites picks its own iteration
  shape: walking a file's roots, scanning a registry, or chaining
  candidates. ADR-0027's principle ("name concrete patterns at their use
  sites; do not generalize") still holds — there is no shared "use-site
  walker" trait.

## Alternatives considered

**1. Use sites stay as `TypeRef` values on declarations; rules walk source.**

Today's shape, plus rules. Each rule re-parses the file with tree-sitter
and walks the tree to find references. Rejected because it duplicates
the parse cost on every diagnostic compute, and because it bypasses
registries entirely — a `TypeRef` value on a declaration cannot subscribe
to its target's lifecycle, so cross-file invalidation has to be wired
ad hoc per consumer. The architecture's registry/RAII machinery does
nothing useful for diagnostics under this option.

**2. Use sites are AST nodes per-file (Roslyn / JDT shape).**

Maintain a per-file AST whose nodes include every use, alongside a
flat cross-file index built from it. The graph is the index; the AST is
the per-file detail. Rejected because beans has no per-file AST product
today — the tree-sitter tree is consumed at parse time and discarded —
and reintroducing one duplicates ownership of source structure between
"the AST" and "the graph." The graph already wants to be the IR; making
it the *cross-file* IR (decls + cross-file uses) and letting per-file
detail come from re-parse keeps one canonical structure.

**3. Use sites are flat index records, not nodes (clangd `libIndex`).**

Maintain a flat `(decl_id, use_path, use_line, use_col)` table keyed by
declaration. Use sites are tuples in this table; they're not nodes,
don't have payloads, don't subscribe. Rejected because the registry
infrastructure (`Rc<RefCell>`, `ProviderHandle`, `Subscription`,
`FallbackSubscription`, snapshot-and-release re-entrancy) was specified
for use sites. If references are tuples, the entire registry layer is
unused for them, and we have two parallel mechanisms for cross-file
flow.

**4. Use sites are query results (Salsa / rust-analyzer demand-driven).**

The graph stores declarations only; "where are the references?" is a
memoized query whose inputs are revision + name. Rejected because it's
a different architecture from beans-as-shipped. ADR-0014/0015's RAII
handles, ADR-0008's fallback subscriptions, and ADR-0017's rejection of
central pipeline machinery presuppose a node-graph shape, not a
query-graph shape. A Salsa-shaped engine is a coherent alternative, but
adopting it is a larger rewrite than this ADR contemplates.

**5. Use sites as nodes, no modifiability axis.**

Add use-site nodes uniformly across modifiable and read-only sources.
Rejected because read-only sources have no consumer for use sites:
neither rename, nor diagnostics, nor find-references operate on
unmodifiable bytecode. Emitting use-site nodes for the JDK alone would
multiply the graph's working set by something on the order of 100x,
without a single query that benefits from the cost.

**6. Use sites as nodes, no asymmetric (locals out) rule.**

Emit a `JavaIdentifierReadNode` for every parameter read, local-var
read, and this-field read. Rejected on sizing: a typical Java service
file has tens of thousands of body-level reads, the workspace has tens
of thousands of files, and the resulting node count is the wrong shape
for an in-memory engine. Local references are answerable by re-parse
at request time (clangd-style); the graph's purpose is cross-file
identity, and intra-method scope is by definition not cross-file. The
modifiability framing makes this distinction follow from a single
principle: graph nodes carry **cross-file/cross-class identity** that
the registry layer can index; everything else is per-file detail.
