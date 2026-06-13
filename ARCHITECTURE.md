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
    graph/               # Layer-1 arena, hard-link forest, RAII handles
    registry/            # Layer-1 typed registries, queries, subscriptions
    jvm/                 # JVM model + JMOD reader
    languages/{java,kotlin,scala,groovy,clojure}/   # feature-gated submodules

beans-lsp/               # LSP server: thin protocol shell over beans-core

beans-test-harness/      # Fixture framework (language-agnostic)
beans-test-jdks/         # Test-only JDK provisioning (download + cache Temurin)
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
beans-core = { default-features = false }
```

There is no `jmod` feature: per ADR-0019 the JVM bytecode side (the
`jvm/` module, including the class container layer) is unconditional —
it serves all languages and survives `default-features = false`.

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

## Layering

Beans is structured as three layers
([ADR-0027](docs/adr/0027-slim-graph-defer-recomputation-to-layer-2.md)):

1. **Data layer** — `Graph<P>` and `Registries`. Storage and indexing of
   typed nodes and the keys they're discoverable under. No analysis logic;
   no recomputation; no lifecycle policy beyond RAII cleanup on destroy.
2. **Analysis layer** — diagnostics, type resolution, dependency analysis.
   Builds on the data layer; owns its own caching, subscription, and
   recomputation patterns. Not yet implemented.
3. **LSP layer** — `beans-lsp`. Builds on layers 1 and 2 to answer client
   requests.

Cross-cutting startup posture: stale-while-revalidate
([ADR-0028](docs/adr/0028-stale-while-revalidate-posture.md)). Last-known
artifacts (squiggles, document symbols, inlay hints) load from snapshot
and display immediately; reconciliation runs in the background. The user
is never gated on the engine catching up.

---

## The semantic graph

The semantic graph is the layer-1 storage substrate. Every artifact beans
produces — diagnostics, completion candidates, hover content, document
symbols — is rooted in a node in this graph or derived from values
ultimately reached through it. The graph itself does no analysis; it is a
typed arena with a hard-link forest and RAII handles, and nothing more
([ADR-0027](docs/adr/0027-slim-graph-defer-recomputation-to-layer-2.md)).

### Nodes

A node is a slot in a flat arena, identified by a generational `NodeId`
([ADR-0007](docs/adr/0007-nodeid-runtime-only-identity.md)). The id pairs
a slot index with the slot's generation at mint time; the slot's
generation bumps every time the slot is freed, so a stale id no longer
matches its slot's current occupant. `NodeId` is runtime-only; not stable
across rebuilds, version upgrades, or any operation that doesn't preserve
the arena byte-for-byte. External APIs never speak in `NodeId`; semantic
identity lives in registry keys.

```rust
struct NodeData<P> {
    payload: P,
    parent: Option<NodeId>,
    children: Vec<NodeId>,              // hard links
    handles: Vec<Box<dyn NodeHandle>>,  // RAII anchors
}
```

There is no `state`, `dynamic_links`, or stability flag on `NodeData`.
Cross-file dependency tracking is mediated by registry watches stored in
`handles`; the graph layer has no per-key knowledge — each handle's `Drop`
does its own cleanup
([ADR-0014](docs/adr/0014-raii-handles-for-subscriptions-and-providers.md)).
When a slot is freed the vec drops, every handle's `Drop` runs, registry
entries vanish.

Node payloads form a layered hierarchy:

- `file://<path>` — a workspace path.
- `cst://<path>` — the tree-sitter parse tree of the file's content.
- Language-model nodes (`java://...`, `kt://...`) — typed representations
  produced by each language module's enrich path.
- JVM projection nodes (`jvm://...`) — the cross-language interop projection.
- External-resource nodes (`dependency://<coord>`, `jmod://<module>`).

LSP-facing artifacts (`diagnostic://<path>`, `document_symbols://<path>`,
`inlay_hints://<path>`) are *not* graph nodes. They are layer-2 consumer
values held outside the graph; their lifetime is the consumer's
subscription handle, not a graph slot.

### Hard links

Hard links are ownership/containment edges within a file's subtree
([ADR-0006](docs/adr/0006-hard-links-and-dynamic-links.md), hard-link half).
A file hard-links its CST; a CST hard-links its language symbols; a Kotlin
symbol hard-links its JVM projection. Hard links are stored as
`Vec<NodeId>` on the parent and never cross file boundaries. The graph is
a forest — multiple roots (one per file, plus per dependency, per JMOD),
each rooting a tree of hard links.

```
file://Service.kt                                      [root]
  └── cst://Service.kt
       └── kt://com.example.Service
            ├── kt://com.example.Service.process
            │    └── jvm://com.example.Service.process       (projection)
            └── kt://java.lang.String.toSlug                 (extension)
                 └── jvm://com.example.ServiceKt.toSlug
```

`Drop` is the GC mechanic. Destroying a node walks its hard-link subtree
post-order, frees every descendant, and bumps each freed slot's generation
so any outstanding `NodeId` resolves to `None` after the destroy. Hard-link
traversal is acyclic by construction (parent set at insert; never
mutated); cycle detection is unnecessary at the graph layer.

### Dynamic dependencies via registry watches

Cross-file dependencies — a use site in `App.java` referencing
`Service.process` defined in `Service.kt` — go through registries, not
graph-level edge fields
([ADR-0008](docs/adr/0008-fallback-queries-on-dynamic-links.md) rev 3). A
use-site node owns a `FallbackSubscription<P, F>` (or a single
`Subscription<K>` for non-fallback cases); its `Watch` lives in the node's
`handles` vec. When the underlying registry's provider set changes, the
watch fires its callback; in practice that callback marks the use-site
stale (a layer-2 concern — the graph itself has no `mark_stale`).

The graph never inspects what its nodes depend on. Watches in `handles`
are just RAII anchors that happen to fire user-supplied callbacks when
invoked.

### What the graph does not do

By design, the graph layer carries no machinery for:

- Per-node cache state (`Fresh`/`Stale`/`Computing`). A graph node holds
  a value; "is the value up to date?" is not a question the graph
  answers.
- Push-stale propagation. Staleness is a layer-2 concept; it travels via
  consumer-owned subscriptions, not via graph edges.
- Pull-recompute orchestration. Layer 2 owns the recompute pattern.
- A stable-vs-volatile node distinction. Stability is a property of
  consumer-held watches: the registry survives volatile churn, so any
  watch into the registry survives too.
- Cycle detection. Hard links are acyclic; registries are O(1) lookups;
  any layer-2 cycle is caught by `RefCell` re-entrancy panic for free.

These were specified by earlier drafts (ADRs 0009/0010/0011); ADR-0027
reverses those at the graph layer and defers them to layer-2 consumers.

---

## Registries

Registries are the substrate for cross-file lookup, subscription, and
invalidation. Every cross-file dependency goes through one.

### Typed per-registry keys

Each registry has its own typed key struct
([ADR-0012](docs/adr/0012-typed-per-registry-keys.md)). There is no shared
`RegistryKey` enum and no generic `Registries::query(key)` entry point.
Resolution code names the registry it is talking to.

```rust
struct JvmMethodKey { owner: Fqn, name: String, params: Vec<TypeRef> }
struct JvmTypeKey   { fqn: Fqn }
struct PackageKey   { name: String }
struct JavaSymbolKey { fqn: Fqn }
struct KotlinExtensionKey { receiver: TypeRef, name: String }
// ...

struct Registries {
    jvm_methods:  Registry<JvmMethodKey>,
    jvm_types:    Registry<JvmTypeKey>,
    jvm_packages: Registry<PackageKey>,

    #[cfg(feature = "java")]   java_symbols:      Registry<JavaSymbolKey>,
    #[cfg(feature = "kotlin")] kotlin_symbols:    Registry<KotlinSymbolKey>,
    #[cfg(feature = "kotlin")] kotlin_extensions: Registry<KotlinExtensionKey>,
    // one field per registry, gated by the relevant language feature
}
```

Wrong-registry queries fail at compile time — a `JvmMethodKey` cannot be
sent to `kotlin_extensions`.

### Multi-provider, no built-in precedence

Registries store **all providers** for each key with no notion of a winner
([ADR-0013](docs/adr/0013-registries-store-all-providers.md)). Java's
classpath shadowing, Kotlin's import precedence, and Clojure's `require`
order are language-specific resolution rules — the registry knows none of
them.

```rust
struct Registry<K> { inner: Rc<RefCell<RegistryInner<K>>> }

struct RegistryInner<K> {
    providers:   HashMap<K, Vec<NodeId>>,
    subscribers: HashMap<K, Vec<(SubscriptionId, Callback)>>,
}
```

Picking a winner is a use-site concern — encoded either in the language
module's resolution code or, for the cross-language fallback case, in
`FallbackSubscription`.

### Query and Subscription: typestate split

Single-key lookups split into two stateful types so the watch/no-watch
lifecycle is enforced at the type level
([ADR-0008 rev 3](docs/adr/0008-fallback-queries-on-dynamic-links.md)):

```rust
// Stateless lookup; just resolve.
struct Query<K> { /* registry handle + key */ }
impl<K> Query<K> {
    fn resolve(&self) -> QueryResult;
    fn subscribe(self, cb: Callback) -> Subscription<K>;
}

// Active subscription; owns a registry entry. Drop unsubscribes.
struct Subscription<K> { /* registry handle + key + id */ }
impl<K> Subscription<K> { fn resolve(&self) -> QueryResult; }
impl<K> Drop for Subscription<K> { /* removes the entry */ }
```

A `Query<K>` is, by construction, not subscribed; a `Subscription<K>` is,
by construction, subscribed. There is no `Option<SubscriptionId>` state
machine inside either type, and no public path that returns a subscribed
handle without subscription enforcement.

`QueryResult` is tri-state and owned. The `NodeId`s it carries are
generational, so a held result is safe to re-check through `graph.get`
after arbitrary mutations:

```rust
enum QueryResult { None, One(NodeId), Many(Vec<NodeId>) }
```

### Cross-language fallback: `FallbackSubscription<P, F>`

The recurring cross-language pattern across all five JVM languages is
exactly two queries: language-native primary plus a JVM fallback. ADR-0008
rev 3 names this directly:

```rust
struct FallbackSubscription<P, F> { /* primary Sub<P>, fallback Sub<F>, cached */ }

impl<P, F> FallbackSubscription<P, F> {
    fn new(reg_p: &Registry<P>, key_p: P,
           reg_f: &Registry<F>, key_f: F) -> Self;
    fn resolve(&self) -> QueryResult;             // primary first, then fallback
    fn subscribe(&self, cb: Callback) -> Watch;   // cache invalidates on either side
}
```

A typical Java-side reference to a method on a `Service` value:

```rust
let fb: FallbackSubscription<JavaSymbolKey, JvmMethodKey> = FallbackSubscription::new(
    &registries.java_symbols, JavaSymbolKey::new("com.example.Service.process"),
    &registries.jvm_methods,  JvmMethodKey::new(owner, "process", params),
);
let watch = fb.subscribe(Rc::new(move || /* mark_stale at use-site */));
```

If Java has the method, the primary wins. If Kotlin defined it (only the
JVM projection exists), the fallback wins. The use site is identical in
both cases.

A future composition that needs a different shape (e.g., completion's
"merge across N registries") gets its own concrete type, named for what
its consumers do — not a generic `MultiQuery<N>`. ADR-0008 rev 3 documents
the rejection.

### RAII handles

Provider registrations return `ProviderHandle<K>`; subscriptions return
`Subscription<K>`; fallbacks return `Watch`. All three impl `NodeHandle`
and live in `NodeData::handles`
([ADR-0014](docs/adr/0014-raii-handles-for-subscriptions-and-providers.md)).
When the node drops, every handle drops, every registry entry vanishes.
Partial-construction failures clean up correctly because the `Vec` is the
only owner.

### `Rc<RefCell<_>>` with snapshot-and-release notify

Registries are `Rc<RefCell<RegistryInner<K>>>`
([ADR-0015](docs/adr/0015-rc-refcell-registry-with-weak-handles.md)). The
graph engine is single-threaded and per-workspace
([ADR-0018](docs/adr/0018-single-threaded-graph-core.md)) — `Arc<Mutex<_>>`
would pay atomic cost we do not need.

Notifications use the snapshot-and-release pattern to keep re-entrant
callbacks safe under `RefCell`:

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

Subscribers added during a callback are picked up on the next
notification, not the current one.

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

deletion  Graph::destroy walks the file's hard-link subtree
          NodeData drops, ProviderHandles drop, registry entries removed
          registry watches fire callbacks at any subscribed use sites
          (e.g., a Java caller's FallbackSubscription); the use-site's
          layer-2 cache invalidates and re-resolves on next read
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

The server creates one graph per workspace. On `initialize`, it loads any
available snapshot, displays last-known artifacts immediately
([ADR-0028](docs/adr/0028-stale-while-revalidate-posture.md)), and queues
revalidation in the background. On `didChange`, tree-sitter performs an
incremental re-parse; the file's volatile subtree is destroyed and
re-integrated; layer-2 caches whose subscriptions fired (squiggles for
this file, dependents in other files) reconcile on next read.

The LSP server holds **layer-2 subscriptions** for proactive artifacts on
open files:

- diagnostics → `textDocument/publishDiagnostics`.
- document symbols → `textDocument/documentSymbol`.
- inlay hints → `textDocument/inlayHint`.

These subscriptions are RAII watches into the appropriate registries; the
registries survive volatile churn, so the watches survive content-clearing
edits like `Ctrl+A, Backspace`.

Per-request handlers are short — look up the relevant nodes, read the
current value (cached or freshly computed), translate into LSP types.
Translation lives in `beans-lsp`; formatting (e.g., hover Markdown) lives
in `beans-core` so a future `beans-cli` can render the same hover content.
Request scheduling, debouncing, and cancellation
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

The current code began as a Java-first `SymbolTable`-based prototype that
informs the architecture but does not constrain it
([ADR-0003](docs/adr/0003-spec-drives-implementation.md)). Where prototype
shape diverges from the architecture described here, the implementation
is the side that moves
([ADR-0021](docs/adr/0021-preserve-tree-sitter-walker-rewrite-layers-above.md)).

Status today:

- Tree-sitter integration lives in `beans-core/src/languages/java/parser.rs`
  and the Java type-reference parser in `types.rs`. Both are **preserved**;
  grammar-quirk knowledge does not become wrong when the model around it
  changes.
- The walker's output is **rewritten** to emit typed node payloads,
  registered through layer-1 registries, with cross-file dependencies
  mediated by registry watches (per ADR-0008 rev 3, ADR-0027). The
  `Symbol`/`SymbolTable` shape from the prototype is gone.
- The `Language` trait is **removed**; languages live as feature-gated
  modules in `beans-core/src/languages/`.

The migration to layer-1 (graph + registries) is substantially complete.
Layer-2 (analysis) and snapshot/fast-restart support remain to be built;
see the backlog for sequencing.
