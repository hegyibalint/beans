# Beans — Architecture

Beans is a Rust library and LSP server that indexes the JVM language family
(Java, Kotlin, Groovy, Scala, Clojure) into a single semantic graph and
answers queries — go-to-definition, completion, references, hover,
diagnostics — across all of them. The library is the product; the LSP server
is one consumer (see [README.md](README.md) for vision and project status).

This document describes **what the system is shaped like today**. Rationale —
*why* it is shaped this way — lives in the [ADRs](docs/adr/README.md). When
this document and an ADR disagree, the ADR wins and this document is wrong;
fix it.

---

## Crate layout

```
beans-core/              # Library: graph engine, JVM model, language modules
  src/
    graph/               # Nodes, registries, hard/dynamic links
    jvm/                 # JVM model + JMOD reader
    lang/{java,kotlin,scala,groovy,clojure}/   # feature-gated submodules

beans-lsp/               # LSP server: thin protocol shell over beans-core

beans-test-harness/      # Fixture framework (language-agnostic)
beans-test-{java,kotlin,scala,groovy,clojure}/   # Per-language spec tests
beans-test-interop/      # Cross-language tests (planned)
```

`beans-core` is one library crate with feature-gated language modules
([ADR-0019](docs/adr/0019-single-core-crate-with-feature-gated-languages.md)).
The node payload union must include every variant any language can produce;
splitting it across crates would force generic boxing or an aggregator crate
that exists only to dodge the cycle. Consumers that need only JVM bytecode
analysis disable the source-language features:

```toml
beans-core = { default-features = false, features = ["jmod"] }
```

`beans-lsp` is a leaf consumer
([ADR-0002](docs/adr/0002-library-first.md),
[ADR-0020](docs/adr/0020-lsp-is-a-leaf-consumer.md)). Nothing depends on it.
It contains LSP wire-protocol handling, request scheduling, and the mapping
between `beans-core` types and `lsp_types`. Everything else — node types,
formatting, resolution — lives in `beans-core` so a future `beans-cli` can
reach the same functionality without going through JSON-RPC.

Test crates mirror the spec structure
([ADR-0022](docs/adr/0022-per-language-test-crates-mirroring-spec-structure.md)):
one per language, plus `beans-test-interop`. Each per-language crate
organises tests by spec chapter
(`beans-test-java/tests/spec/jls08_classes.rs`); opening a chapter file
shows what beans claims about that chapter.

The architecture is *cohesive, not extensible*
([ADR-0001](docs/adr/0001-cohesive-not-extensible.md)). The five target
languages are baked in at compile time. There is no plugin API and no runtime
registry of "any future language."

---

## Core data model

### `TypeRef`

A structured representation of types — substitution, erasure (JLS §4.6),
subtype checking, cross-file semantic analysis.

```rust
enum TypeRef {
    Void,
    Primitive(PrimitiveKind),
    Simple { name: String },          // resolved or unresolved class name
    Parameterized { raw: Box<TypeRef>, args: Vec<TypeRef> },
    TypeVariable { name: String },
    Wildcard { bound: Option<WildcardBound> },
    Array { element: Box<TypeRef> },
    Intersection { types: Vec<TypeRef> },
    Unknown,                          // sentinel; propagates without panic
}
```

`TypeRef` is the lingua franca across all languages. Per-language
refinements (Kotlin nullability, Scala HKT) live on the surrounding language
node, not on `TypeRef`.

### Node payloads

The graph stores typed payloads — there is no monolithic `Symbol`. Each
language module defines payload variants for its kinds; `beans-core` defines
the JVM payload variants. The union enum is internal and gated by language
features.

Common fields on every payload that can carry them: `name`, `fqn`,
`location: Option<Location>`, `modifiers: Vec<Modifier>`,
`annotations: Vec<AnnotationInstance>`. Method-shaped payloads carry
parameters, return type, type parameters, and `throws` directly; field-
shaped payloads carry a type and an optional `ConstantValue`; record-shaped
payloads carry `RecordComponent`s. Payload shape is determined by variant —
there is no `signature: Option<Signature>` escape hatch
([ADR-0021](docs/adr/0021-preserve-tree-sitter-walker-rewrite-layers-above.md)).

### `AnnotationInstance`

```rust
struct AnnotationInstance {
    fqn: String,                                  // e.g. "java.lang.Override"
    elements: Vec<(String, AnnotationValue)>,     // JLS 9.6.1
}

enum AnnotationValue {
    Const(ConstantValue),
    ClassLiteral(TypeRef),
    EnumRef { type_fqn: String, constant: String },
    Annotation(Box<AnnotationInstance>),
    Array(Vec<AnnotationValue>),
}
```

Annotations are first-class on every payload that can carry them. Diagnostic
rules and JVM enrichments (e.g., nullability) read these directly.

---

## The semantic graph

The semantic graph is the system's core computation engine
([ADR-0006](docs/adr/0006-hard-links-and-dynamic-links.md)). Every artifact
beans produces — diagnostics, completion candidates, hover content, document
symbols — is a node in this graph or derived from one.

### Nodes

A node is a slot in a flat arena, identified by a `NodeId` (a `u64`).
`NodeId` is a runtime arena index; it is **not** stable across rebuilds,
version upgrades, or any operation that doesn't preserve the arena
byte-for-byte
([ADR-0007](docs/adr/0007-nodeid-runtime-only-identity.md)). External APIs
never speak in `NodeId`; semantic identity lives in registry keys.

```rust
struct NodeData {
    state: CacheState,            // Fresh(generation) | Stale | Computing
    payload: NodePayload,
    parent: Option<NodeId>,
    children: Vec<NodeId>,        // hard links
    dynamic_links: Vec<DynamicLink>,
    providers: Vec<ProviderHandle>,
    subscriptions: Vec<SubscriptionHandle>,
}
```

Node payloads form a layered hierarchy:

- `file://<path>` — a workspace path. Stable; persists through edits.
- `cst://<path>` — the tree-sitter parse tree of the file's content.
- Language-model nodes (`java://...`, `kt://...`) — typed representations
  produced by the language module's enrich path.
- JVM projection nodes (`jvm://...`) — the cross-language interop projection.
- View nodes (`diagnostic://<path>`, `document_symbols://<path>`,
  `inlay_hints://<path>`) — LSP-facing computed views.
- External-resource nodes (`dependency://<coord>`, `jmod://<module>`).

### Hard links

Hard links are ownership/containment edges within a file's subtree. A file
hard-links its CST; a CST hard-links its language symbols; a Kotlin symbol
hard-links its JVM projection. Hard links are stored as `Vec<NodeId>` on the
parent. When the parent is destroyed, the GC walks the tree top-down and
destroys every child. No registry is involved — hard links are private,
deterministic, and never cross file boundaries.

```
file://Service.kt
  └── cst://Service.kt
       └── kt://com.example.Service
            ├── kt://com.example.Service.process
            │    └── jvm://com.example.Service.process       (projection)
            └── kt://java.lang.String.toSlug                 (extension)
                 └── jvm://com.example.ServiceKt.toSlug
```

### Dynamic links

Dynamic links are cross-file dependency edges, mediated by registries
([ADR-0008](docs/adr/0008-fallback-queries-on-dynamic-links.md)). A use site
in `App.java` referencing `Service.process` does not store a target `NodeId`;
it stores an ordered list of registry queries plus a cached result for
whichever query is currently active.

```rust
struct DynamicLink {
    queries: Vec<RegistryQuery>,    // ordered, first-match wins (or merge-all)
    mode: LinkMode,                 // FirstMatch | MergeAll
    active_index: Option<usize>,
    cached_result: Option<NodeId>,
}
```

Two combine modes:

- **`FirstMatch`** — go-to-definition, type-checking. The first query that
  hits provides the value; lower-priority queries are not consulted.
- **`MergeAll`** — completion. Every query fires and the results union, with
  language-specific candidates winning over JVM projections for the same
  symbol.

Subscriptions are tiered by query position: the active query has a
**value-watch**, higher-priority queries (currently missing) have
**existence-watches** (fire if a hit appears that would supersede), and
lower-priority queries are unobserved while a higher one is active.

When the user moves a definition between languages, the use site's query
list does not change; only the active index moves.

### Stable vs volatile nodes

Nodes have lifecycles tied to different things
([ADR-0011](docs/adr/0011-stable-vs-volatile-nodes.md)):

- **Stable** — identity tied to an external resource that outlives content
  snapshots. `file://`, `dependency://`, `jmod://`, and LSP view nodes
  (`diagnostic://Service.kt`). `NodeId`s are preserved across content
  changes; cached values may update but the slot persists. This is what
  allows the LSP client to keep a long-lived subscription handle to
  `diagnostic://Service.kt` across `Ctrl+A, Backspace, Ctrl+Z` without
  re-registering.
- **Volatile** — derived from content; recreated when content changes. CSTs,
  language symbols, JVM projections. Destroyed by the hard-link GC walk.

A stable file node hard-links volatile children (CST, language nodes) and
stable view nodes (`diagnostic://`). When content clears, the volatile
subtree is destroyed; the file and view nodes persist; the client's handle
stays valid.

### Push-stale, pull-recompute

Invalidation is two-phase
([ADR-0009](docs/adr/0009-push-stale-pull-recompute.md),
[ADR-0010](docs/adr/0010-lazy-recomputation.md)):

1. **Push (eager, cheap).** When a file changes, tree-sitter diffs identify
   affected CST nodes. Registries notify subscribers, marking them stale.
   Staleness propagates through dynamic links. Marking is a flag flip; no
   value is computed.
2. **Pull (lazy, on demand).** When something requests a value, the graph
   walks down from the requested node. Fresh nodes return cached values.
   Stale nodes recompute, recursively pulling dependencies. Only the actual
   ancestor chain is touched.

```
pull(node):
    match node.state:
        Fresh(_)  -> return node.value
        Computing -> cycle; return partial or error
        Stale     -> node.state = Computing
                     for dep in node.dependencies: pull(dep)
                     node.value = recompute(node)
                     node.state = Fresh(current_generation)
                     return node.value
```

A stale node that nobody pulls stays stale forever. Closed-file
diagnostics nobody reads cost nothing.

### Snapshot save/load

The arena, hard links, dynamic-link queries, and registry subscriber lists
serialise to a binary snapshot. On startup the snapshot is deserialised
(sub-second), files changed since are marked stale, and the LSP starts
answering queries; the next pull on a stale node walks its chain and
refreshes. 99% of the graph is immediately usable; re-parsing happens on
demand rather than at startup. Missing dependencies (a JAR that was on the
path last session but is gone now) mark dependents stale rather than
crashing.

The format is opaque (bincode or rkyv) and versioned; backward compatibility
is not guaranteed. `NodeId`s round-trip verbatim because the entire arena
is preserved.

---

## Registries

Registries are the substrate for cross-file lookup, subscription, and
invalidation. Every dynamic link goes through a registry.

### Typed per-registry keys

Each registry has its own typed key struct
([ADR-0012](docs/adr/0012-typed-per-registry-keys.md)). There is no shared
`RegistryKey` enum and no generic `Registries::query(key)` entry point.
Resolution code names the registry it is talking to.

```rust
struct JvmMethodKey { owner: ClassFqn, name: String, params: Vec<JvmDescriptor> }
struct KotlinExtensionKey { receiver: TypeRef, name: String }
struct ScalaImplicitKey { target: TypeRef }
// ...

struct Registries {
    jvm_methods: Registry<JvmMethodKey>,
    jvm_fields:  Registry<JvmFieldKey>,
    packages:    Registry<PackageKey>,
    modules:     Registry<ModuleKey>,

    #[cfg(feature = "java")]    java: Registry<JavaSymbolKey>,
    #[cfg(feature = "kotlin")]  kotlin: Registry<KotlinSymbolKey>,
    #[cfg(feature = "kotlin")]  kotlin_extensions: Registry<KotlinExtensionKey>,
    #[cfg(feature = "scala")]   scala_implicits: Registry<ScalaImplicitKey>,
    // ... one field per registry, gated by the relevant language feature
}
```

Wrong-registry queries fail at compile time. A `JvmMethodKey` cannot be sent
to `kotlin_extensions`; the types do not match.

### Multi-provider, no built-in precedence

Registries store **all providers** for each key, with no notion of a winner
([ADR-0013](docs/adr/0013-registries-store-all-providers.md)). Java's
classpath shadowing, Kotlin's import precedence, and Clojure's `require`
order are language-specific resolution rules — the registry knows none of
them.

```rust
struct Registry<K> { inner: Rc<RefCell<RegistryInner<K>>> }

struct RegistryInner<K> {
    providers:   HashMap<K, Vec<NodeId>>,
    subscribers: HashMap<K, Vec<SubscriberEntry>>,
}
```

`Registry::query(key)` returns the entire provider list. Picking a winner is
a resolution-layer concern, implemented in the language module that knows
the rules for the call site.

### Fallback queries

Cross-language resolution uses fallback queries on dynamic links rather than
registry-internal precedence. A Java reference to `process` on a `Service`
value carries:

```
[
  java::JavaSymbolKey { owner: "com.example.Service", name: "process" },
  jvm::JvmMethodKey   { owner: "com.example.Service", name: "process", params: [..] },
]
```

If Java has the method, the first query wins. If a Kotlin-defined `Service`
projects to JVM, the second wins. Subscriptions adjust automatically as
files appear and disappear.

### RAII handles

Provider registrations and subscriptions are returned as owning handles
whose `Drop` impl performs cleanup
([ADR-0014](docs/adr/0014-raii-handles-for-subscriptions-and-providers.md)).
`NodeData` holds them in `Vec`s; when a node is dropped — for any reason, on
any path — its handles drop, and each handle's `Drop` removes the registry
entry. There is no separate `on_destroyed` cleanup the GC has to remember to
call. Partial-construction failures clean up correctly.

### `Rc<RefCell<_>>` with `Weak` back-references

Registries are `Rc<RefCell<RegistryInner<K>>>`; handles hold a
`Weak<RefCell<RegistryInner<K>>>`
([ADR-0015](docs/adr/0015-rc-refcell-registry-with-weak-handles.md)). The
graph engine is single-threaded and multi-instance (each LSP workspace has
its own graph), so `Arc<Mutex<_>>` would pay atomic cost we do not need
and `'static` singletons would forbid isolation. `Weak` back-references
prevent reference cycles between the registry and node-owned handles; when
the graph tears down, the registry can drop while handles still exist,
and their `Drop` impls degrade gracefully via the failed `Weak::upgrade`.

Notifications follow the **snapshot-and-release** pattern to keep
re-entrant callbacks safe under `RefCell`:

```rust
fn notify(&self, key: &K) {
    let subscribers: Vec<Callback> = {
        let inner = self.inner.borrow();
        inner.subscribers.get(key).cloned().unwrap_or_default()
    };
    // borrow released; callbacks may re-enter the registry
    for cb in subscribers { cb.run(); }
}
```

Subscribers added during a callback are picked up on the next notification,
not the current one.

---

## Lifecycle

### Synchronous enrich-then-register

Every node is fully formed before it enters the graph
([ADR-0016](docs/adr/0016-pipeline-phases-before-registration.md)). Each
language module has an `enrich` path that runs as a regular synchronous
function: resolve types against imports, apply language-specific
enrichments, compute the JVM projection, generate descriptors. Only then is
the node registered.

```rust
impl KotlinClassNode {
    fn enrich(parsed: ParsedKotlinClass, ctx: &mut Context) -> Self {
        let supertypes  = resolve_types(&parsed.supertypes, ctx);
        let nullability = apply_kotlin_nullability(&parsed, ctx);
        let projection  = jvm_utils::project_class(&parsed, &supertypes);
        Self { supertypes, nullability, projection, /* ... */ }
    }
}
```

There are **no event-driven processors** that subscribe to registry events
and mutate nodes. A processor model produces re-trigger loops where
execution order is emergent. Synchronous enrich makes that shape impossible
by construction.

Cross-file dependencies are handled by the dynamic-link mechanism, not by
mid-enrich blocking. If `App.java` references `Service.process` and
`Service.kt` has not been parsed yet, the Java node's link resolves to
"no match"; when `Service.kt` is later parsed and registered, registry
notification marks the Java node stale, and the next pull re-resolves.

### No central pipeline

Each node type owns its enrich function
([ADR-0017](docs/adr/0017-no-central-pipeline-machinery.md)). There is no
`Pipeline` trait, no phase registry, no `EnrichmentPhase` trait. The five
languages do *roughly* the same shape of work, but the differences are
precisely what make each language non-trivial. Shared work is exposed as
utility functions, not phases:

```rust
jvm_utils::erasure(type_ref) -> JvmType
jvm_utils::descriptor_from_signature(sig) -> String
jvm_utils::project_class(parsed, supertypes) -> JvmProjection
```

Each language's enrich function calls the utilities it needs in the order
that makes sense for that language. If two languages later need the *same*
phase (same input, same output), it is extracted as another utility — not
as a composable phase.

### Worked example: a Kotlin extension function

```
parse     tree-sitter walks Service.kt; walker reaches `fun String.toSlug()`
enrich    resolve receiver TypeRef ("String" → "kotlin.String")
          resolve return type
          compute JVM projection (synthetic class "ServiceKt", method "toSlug")
          build the KotlinExtensionFunction node value
allocate  NodeId for the Kotlin node; NodeId for its JVM projection (hard-linked)
register  provider in `kotlin_extensions` (receiver + name)
          provider in `jvm_methods` (owner + name + params)
          RAII handles stored on NodeData

deletion  push-stale walks the file's volatile subtree
          NodeData drops, ProviderHandles drop, registry entries removed
          subscribers (e.g., a Java caller) notified → marked stale
          next pull on the Java caller finds no provider; emits "unresolved"
```

---

## JVM projection layer

A universal model expressive enough for every JVM language's type system is
either lowest-common-denominator or maximalist; both lose. Instead, **each
language has its own rich model**, and cross-language interop goes through
a shared **JVM projection**
([ADR-0004](docs/adr/0004-per-language-models-with-jvm-projection.md)).

```
Language-specific models  (Kotlin nullability/properties/extensions,
                           Scala HKT/implicits, Groovy closures/MOP,
                           Clojure namespaces/protocols, Java inference)
                ↓  each language-model node hard-links a JVM projection
JVM projection (shared)   classes, methods, fields, constructors
                          generic signatures + erasure
                          promoted enrichments (nullability, ...)
```

The Kotlin class `com.example.Service` produces `kt://com.example.Service`
plus `jvm://com.example.Service` as a hard-linked descendant. Within-
language operations (Kotlin completion in a Kotlin file) walk the rich
model. Cross-language operations (Java calling that Kotlin class) go
through the JVM projection.

### Promoted enrichments

The JVM projection carries a small set of universally valuable enrichments
lifted from the language models:

```rust
struct JvmEnrichments {
    nullability: Option<NullabilityInfo>,    // Kotlin types, Java @Nullable, Scala
    property_origin: Option<PropertyOrigin>, // Kotlin/Groovy properties, Scala vals
    has_defaults: Vec<bool>,                 // default parameter values
}
```

Promotion is explicit and minimal. Only features with cross-language
consumers leak into JVM: Kotlin nullability promotes (Java benefits from
knowing a Kotlin method returns non-null), but Kotlin extension functions
stay in `kotlin_extensions` (no other language has the same dispatch
model).

---

## Concurrency

The graph core is **single-threaded**
([ADR-0018](docs/adr/0018-single-threaded-graph-core.md)). All mutation of
`NodeData`, `Registry`, hard links, and dynamic links happens on one
thread, using `Rc<RefCell<_>>`. There is no `Mutex`, `RwLock`, or `Arc`
inside the graph.

Parallelism comes from `rayon` at the file-batch boundary
([ADR-0005](docs/adr/0005-sync-core-rayon-parallelism.md)). Files parse in
parallel on the rayon pool; each parse produces a self-contained
`ParsedFile` with no graph references. The graph thread integrates the
batch serially — enrich, allocate, register. The serial integration step is
cheap relative to the parse it follows.

```
┌── thread pool (rayon) ──┐         ┌── graph thread ──┐
   parse file A  ──┐
   parse file B  ──┼── batch ────►   integrate(A, B, C)
   parse file C  ──┘                   (one at a time)
```

The LSP boundary is async because `tower-lsp` is async; the server calls
into the sync core. No async colours leak inwards. There is no async
runtime in `beans-core`.

`RefCell` borrow violations are bugs to fix, not recoverable conditions;
the fix path is the snapshot-and-release pattern. LSP request handlers are
wrapped in `std::panic::catch_unwind` so a panic in one handler returns an
error to the client without killing the server.

---

## LSP integration

`beans-lsp` wires `beans-core`'s graph to the LSP protocol. It is a leaf in
the dependency graph; nothing depends on it.

The server creates one graph per workspace. On `initialize`, it discovers
source roots, schedules initial parsing on the rayon pool, and integrates
results. On `didChange`, tree-sitter performs an incremental re-parse, the
diff is push-staled, and the next pull recomputes whatever the next
request asks for.

The LSP server **subscribes to view nodes** for open files:

- `diagnostic://<path>` — `textDocument/publishDiagnostics`.
- `document_symbols://<path>` — `textDocument/documentSymbol`.
- `inlay_hints://<path>` — `textDocument/inlayHint`.

View nodes are stable; the LSP's subscription handle survives content-
clearing edits like `Ctrl+A, Backspace`.

Per-request handlers are short — look up a node, call `pull`, translate the
result into LSP types. Translation lives in `beans-lsp`; formatting (e.g.,
hover Markdown) lives in `beans-core` so a future `beans-cli` can render
the same hover content. Request scheduling, debouncing, and cancellation
live in `beans-lsp`. The graph itself does not know about LSP; it knows
about pulls and stales.

---

## Testing

Tests are per-language crates mirroring the spec structure
([ADR-0022](docs/adr/0022-per-language-test-crates-mirroring-spec-structure.md)).
Each file maps to a chapter of the language spec; sub-modules map to
sections (`mod jls_7_5_1_single_type_import`). Tests cite the spec in a
comment or via the module name.

The fixture framework (`beans-test-harness`) is the primary way to encode
expected behavior. Two operations:

- `.complete(|items| { ... })` — completion at a `<cur>` marker.
- `.resolve()` — go-to-definition / hover at a `<cur>` marker.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the fixture tutorial and the test discipline.

Spec tests are mass-authored by LLM agents and reviewed by humans
([ADR-0023](docs/adr/0023-mass-author-spec-tests-via-llm-agents-with-human-review.md));
volume over precision in the initial pass.

Every spec test starts marked `expected_failure`
([ADR-0024](docs/adr/0024-tests-start-expected-failure-and-prefer-negative-spec-violations.md)).
The marker is a bookmark — "this is what we owe the spec" — that keeps CI
green while the engine catches up. The framework treats an unexpected pass
as a CI failure, so promotion is a positive signal. Tests prefer **specific
assertable facts** over absence; "valid code produces no diagnostics" is
the canonical trivial-passer (it goes green against an engine that does
nothing).

CI runs the suite twice
([ADR-0025](docs/adr/0025-dual-mode-check-real-engine-vs-empty-engine.md)):
once against the real engine, once against an "empty engine" whose parsers
register nothing. A test that passes in *both* modes isn't exercising the
engine; CI flags it. Local runs use only the real-engine mode for speed.

Tests that legitimately depend on absence opt out with a per-test marker
that takes a justification string
([ADR-0026](docs/adr/0026-per-test-opt-out-for-absence-dependent-tests.md)):

```rust
.absence_dependent("missing import: absence is the assertable fact")
```

The marker is rare by design; a spike of `absence_dependent` markers in a
PR is a review prompt.

See [CONTRIBUTING.md](CONTRIBUTING.md) for authoring conventions.

---

## Migration status

The current code is a Java-first prototype that informs the architecture
but does not constrain it
([ADR-0003](docs/adr/0003-spec-drives-implementation.md)). Where the
prototype's `SymbolTable`-based design diverges from the architecture
described here, the implementation is the side that moves
([ADR-0021](docs/adr/0021-preserve-tree-sitter-walker-rewrite-layers-above.md)):

- Tree-sitter integration in `beans-lang-java/src/parser.rs` and the Java
  type-reference parser in `types.rs` are **preserved**. Grammar-quirk
  knowledge does not become wrong when the model around it changes.
- The output of the walker is **rewritten**. `Symbol`/`SymbolTable` emission
  is replaced by typed node payloads with hard and dynamic links. The
  walker's `extract_*` functions keep their tree-sitter signatures; only
  the body that builds the result changes.
- The `Language` trait is **removed**. Its place is taken by feature-gated
  language modules in `beans-core`.

The migration is incremental at the function-by-function level inside the
walker, not at the module level.
