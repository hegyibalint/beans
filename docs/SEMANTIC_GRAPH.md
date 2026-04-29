# Semantic Graph Architecture

The semantic graph is beans' core computation engine. It powers diagnostics, type checking, completions, and cross-language resolution through a pull-based, cacheable, serializable DAG with push-invalidation.

## Core Concepts

### Two Types of Edges

**Hard links** — ownership/containment within a file's subtree. If parent dies, children die. Deterministic, no registry involved.

```
file://Service.kt
  └── cst://Service.kt           (tree-sitter parse tree)
       └── kt://Service           (Kotlin model node)
            ├── kt://Service.process
            │    └── jvm://Service.process   (JVM projection, hard-linked)
            └── kt://String.toSlug
                 └── jvm://ServiceKt.toSlug  (JVM projection, hard-linked)
```

**Dynamic links** — cross-file dependencies mediated by registries. These are the "soft" edges that survive file deletion/restoration. A dynamic link carries an ordered list of **fallback queries** — first match wins.

```
java://App.run has a dynamic link:
  queries: [
    Q1: JavaRegistry("Service.process")    → miss
    Q2: JvmRegistry("com.example.Service.process") → HIT
  ]
  active_index: 1 (Q2 is providing the value)
```

### Push-Stale, Pull-Recompute

**Push phase** (cheap, no computation): when a file changes, tree-sitter diffs identify which nodes are affected. Those nodes are marked stale. The registries notify subscribers, propagating staleness upward through dynamic links.

**Pull phase** (lazy, on demand): when the LSP needs a value (e.g., diagnostics for a file), it pulls from the top of the graph. Fresh nodes short-circuit. Stale nodes recompute, pulling from their dependencies recursively.

### Tiered Subscriptions on Dynamic Links

A dynamic link with fallback queries `[Q1, Q2, Q3]` where Q2 is active:

- **Q1**: existence watch only (higher priority, currently missing — fires if Q1 appears)
- **Q2**: value watch (active — fires when the value changes)
- **Q3**: nothing (lower priority, irrelevant while Q2 is active)

This ensures no redundant notifications. If Q1 appears later, the existence watch fires, the chain is re-resolved, and subscriptions are updated.

For **completion** (not resolution), dynamic links use **MergeAll** mode — all queries contribute, results are deduplicated with language-specific results winning over JVM for the same symbol.

## Per-Language Models + JVM Interop Layer

### The Problem

A universal model (one TypeRef, one Symbol for all languages) produces mediocre coverage everywhere. Java gets ~75%, Kotlin ~40%, Scala ~25%. Each language has type system features that don't fit a common denominator (Kotlin nullability, Scala HKT, Groovy closure delegation).

### The Solution: Language-Specific Models with JVM Projection

Each language has its own rich model and registries. Cross-language interop goes through a shared JVM layer.

```
┌─────────────────────────────────────────────┐
│         Language-Specific Models             │
│                                             │
│  Kotlin: nullability, extensions, properties│
│  Scala:  HKT, path-dependent, implicits    │
│  Groovy: closures, delegation, MOP         │
│  Clojure: namespaces, vars, protocols      │
│  Java:   type inference, poly expressions  │
├─────────────────────────────────────────────┤
│         JVM Layer (interop)                 │
│                                             │
│  Classes, methods, fields, constructors    │
│  Generics (signature + erasure)            │
│  Promoted enrichments (nullability, etc.)  │
└─────────────────────────────────────────────┘
```

**Within-language**: Kotlin→Kotlin uses the rich Kotlin model. Full fidelity.

**Cross-language**: Kotlin→Java or Java→Kotlin goes through the JVM layer. Each language-specific node **projects** itself to a JVM node (hard-linked child). The projection includes **promoted enrichments** — information so universally valuable that it belongs in the JVM layer (e.g., nullability).

### Promoted Enrichments

Not everything from the language model leaks into JVM. Only things with universal cross-language value:

```rust
struct JvmEnrichments {
    /// Nullability per param/return. From Kotlin's type system,
    /// Java's @NonNull, Scala's annotations. Every language benefits.
    nullability: Option<NullabilityInfo>,

    /// Is this actually a property (with getter/setter convention)?
    /// Kotlin properties, Groovy properties, Scala vals.
    property_origin: Option<PropertyOrigin>,

    /// Default parameter values exist?
    has_defaults: Vec<bool>,
}
```

### Cross-Language Resolution via Fallback Queries

When Java code references a Kotlin method:

```
1. JavaRegistry("Service.process") → miss (it's not a Java symbol)
2. JvmRegistry("com.example.Service.process") → hit (JVM projection of Kotlin node)
   → JVM signature + promoted enrichments (nullability info)
```

When Kotlin code uses an extension function:

```
1. KotlinExtensionRegistry(receiver: String, name: "toSlug") → hit
   → Kotlin-native node, full extension semantics
   (never falls through to JVM)
```

## Node Behavior — Explicit Registry Interaction

Each node type explicitly knows which registries it interacts with. No magic discovery, no scanning.

```rust
trait GraphNodeBehavior {
    /// Register this node as a provider in the appropriate registries.
    fn on_created(&self, id: NodeId, registries: &mut Registries);

    /// Exact reverse of on_created. Unregister from all registries.
    fn on_destroyed(&self, id: NodeId, registries: &mut Registries);

    /// Called when this node's value is recomputed.
    /// Updates registry values, which may trigger subscriber notifications.
    fn on_updated(&self, id: NodeId, registries: &mut Registries);

    /// Hard-link children for GC tree walk.
    fn children(&self) -> &[NodeId];
}
```

Example — a Kotlin extension function node:

```
on_created:
  1. registries.kotlin.register(fqn, self.id)
  2. registries.kotlin_extensions.register(receiver_type, self.id)
  3. project to JVM → registries.jvm.register(jvm_fqn, jvm_node_id)

on_destroyed:
  1. registries.jvm.unregister(jvm_fqn)
  2. registries.kotlin_extensions.unregister(receiver_type, self.id)
  3. registries.kotlin.unregister(fqn)
```

GC walks the hard-link tree top-down, calling `on_destroyed` on each node. Each node unregisters from exactly the registries it registered in. The registries handle notifying subscribers.

## Registries

### Structure

```rust
struct Registry<K> {
    providers: HashMap<K, NodeId>,
    subscribers: HashMap<K, Vec<Subscription>>,
}

struct Subscription {
    subscriber: NodeId,
    kind: SubscriptionKind, // Value | Existence
}
```

The registry owns dependency tracking. Nodes do not track their dependents — registries handle all notification routing.

### Registry Inventory

**Shared (all languages):**
- `JvmRegistry` — FQN → JVM symbol node (the interop layer)
- `PackageRegistry` — package name → member nodes
- `ModuleRegistry` — JPMS module → ModuleInfo node

**Kotlin-specific:**
- `KotlinRegistry` — FQN → Kotlin symbol node
- `KotlinExtensionRegistry` — receiver TypeRef → extension nodes
- `KotlinCompanionRegistry` — class FQN → companion node

**Scala-specific:**
- `ScalaRegistry` — FQN → Scala symbol node
- `ScalaImplicitRegistry` — target type → given/implicit nodes
- `ScalaExtensionRegistry` — receiver TypeRef → extension nodes

**Groovy-specific:**
- `GroovyRegistry` — FQN → Groovy symbol node
- `GroovyDelegateRegistry` — closure scope → delegate type

**Clojure-specific:**
- `ClojureRegistry` — ns-qualified name → var node
- `ClojureProtocolRegistry` — protocol FQN → protocol node

**Java-specific:**
- `JavaRegistry` — FQN → Java symbol node

## Graph Node Structure

```rust
struct GraphNode {
    id: NodeId,
    state: CacheState,          // Fresh(generation) | Stale | Computing
    value: NodeValue,           // layer-specific cached value
    layer: NodeLayer,           // File | Cst | LanguageModel | JvmProjection | Diagnostic

    // Hard links (ownership tree)
    parent: Option<NodeId>,
    children: Vec<NodeId>,

    // Dynamic links (outgoing queries with fallback)
    dynamic_links: Vec<DynamicLink>,
}

struct DynamicLink {
    queries: Vec<RegistryQuery>,     // ordered, first-match wins
    mode: LinkMode,                  // FirstMatch | MergeAll
    active_index: Option<usize>,     // which query currently provides the value
    cached_result: Option<NodeId>,   // resolved node (or None if all miss)
}

enum CacheState {
    Fresh(Generation),
    Stale,
    Computing, // cycle detection
}
```

## Delete/Restore Cycle

The registry subscriber list survives provider death, enabling automatic reconnection.

**Delete:**
1. File content cleared → tree-sitter returns empty tree
2. Hard-link tree walked top-down → each node calls `on_destroyed`
3. Registries unregister providers, notify VALUE subscribers (→ stale)
4. Registries keep subscriber entries (consumers still want this key)

**Restore:**
1. File content restored → tree-sitter re-parses
2. New nodes created → each calls `on_created`
3. Registries register providers, check subscriber lists
4. Subscribers notified → stale → next pull reconnects

## Serialization / Warm Restart

The entire graph (nodes, hard links, dynamic links, registry subscriber lists) is serializable to a binary format.

**Startup:**
1. Deserialize graph from disk → instant (sub-second)
2. Diff files changed since last snapshot → mark affected nodes stale
3. Background: re-parse stale files → push invalidation → pull on demand
4. Result: 99% of the graph is immediately usable

**Missing dependencies:** If a dependency JAR is missing at load time, nodes depending on it are marked stale (not crashed). Diagnostics show "unresolved" for those symbols. Everything else works.

**Format:** Versioned binary (bincode or rkyv). Opaque — can evolve without backward compatibility. Future: wire-compatible format for remote caching.

## Diagnostic Rules

Rules are pure functions that pull from registries via a `RuleContext`. They don't know about the graph — they just ask questions and get answers.

```rust
trait DiagnosticRule: Send + Sync {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic>;
}

struct RuleContext<'g> {
    graph: &'g SemanticGraph,
    target: NodeId,
}

impl<'g> RuleContext<'g> {
    fn symbol(&self, fqn: &str) -> Option<&JvmSymbol> { ... }
    fn supertypes(&self, fqn: &str) -> Vec<String> { ... }
    fn package_members(&self, pkg: &str) -> Vec<String> { ... }
    fn kotlin_extensions(&self, receiver: &TypeRef) -> Vec<NodeId> { ... }
    // Each call through RuleContext registers a subscription
}
```

Each `RuleContext` method call registers a subscription in the relevant registry. The rule author never thinks about caching or invalidation.

## Performance Characteristics

- **Node creation/destruction**: O(k) where k = number of registries the node type uses (1-3)
- **Notification**: O(s) where s = subscribers for that specific key
- **Dynamic link resolution**: O(q) where q = queries in the fallback chain (1-2 typically)
- **Stale marking**: O(1) per node (flip a flag)
- **Recomputation**: only on pull, only for the stale path
- **Startup**: O(file_diff_size), not O(project_size)
