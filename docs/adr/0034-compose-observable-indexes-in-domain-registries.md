# ADR-0034: Compose observable indexes in domain registries

## Status

Accepted

## Context

ADR-0012 established that registries use typed keys and are addressed by
named fields, not by a shared registry-key enum. ADR-0013 established that
registries store all providers and leave precedence to resolution. ADR-0014
and ADR-0015 established the RAII and `Rc<RefCell<_>>` mechanics that make
provider cleanup and subscriptions automatic.

Those decisions still leave an important boundary question: what happens when
one domain concept needs more than one lookup mode?

JVM types are the first concrete pressure. They need exact lookup by fully
qualified type key, and they also need lookup by source simple name for
missing-import and candidate discovery flows. Both lookups are observable:
callers should be able to query and subscribe to either one. Java symbols have
similar pressure.

One tempting answer is to make the generic registry primitive support
secondary indexes. That leads toward abstractions such as `SimpleNamed`,
`NamedRegistry<K>`, `Queryable`, `Subscribable`, projections, or eventually a
generic observable database. This is too much abstraction for the current
system, and it puts language/JVM semantics into `beans-core`, whose job is to
stay neutral.

## Decision

`beans-core::Registry<K>` remains the only generic observable index primitive.
It owns exact-key provider lookup, subscriptions, notification batching, and
RAII provider handles. It does not know about simple names, FQNs, Java symbols,
JVM type names, secondary indexes, or projections.

Domain crates compose one or more concrete `Registry<K>` instances behind
well-named domain registry structs. For example:

```rust
pub struct JvmRegistries {
    pub types: JvmTypeRegistry,
    pub methods: JvmMethodRegistry,
    pub fields: JvmFieldRegistry,
    pub constructors: JvmConstructorRegistry,
    pub packages: JvmPackageRegistry,
}
```

`JvmTypeRegistry` may contain multiple observable indexes:

```rust
pub struct JvmTypeRegistry {
    by_fqn: Registry<JvmTypeKey>,
    by_simple_name: Registry<JvmTypeSimpleNameKey>,
}
```

Both indexes are real registries. Both can be queried and subscribed to. The
domain registry owns the lifecycle coordination between them and returns a
compound RAII provider handle when one registration needs to update more than
one concrete registry.

Exact-only registries still get domain structs, even when they are initially
thin wrappers around one `Registry<K>`. The wrapper is valuable because the
registry's semantics are important and likely to grow.

We will prefer inherent methods on the domain registry structs over generic
`Queryable` / `Subscribable` traits until a concrete generic algorithm earns
those traits.

## Consequences

**Positive.**

- `beans-core` stays neutral. It provides the observable-index primitive but
  does not learn JVM or source-language naming concepts.
- Registry APIs communicate domain capability. A `JvmTypeRegistry` can expose
  simple-name lookup; a `JvmMethodRegistry` does not accidentally inherit that
  surface just because it is backed by the same primitive.
- Multi-index lifecycle is explicit. Domain registries decide how to register,
  unregister, batch, and notify across their concrete indexes.
- Thin domain wrappers provide stable homes for future complexity without
  forcing generic abstractions early.
- The design remains compatible with graph-owned RAII cleanup: nodes still own
  boxed handles, and domain handles can drop multiple provider handles.

**Negative.**

- There is more boilerplate. Each registry gets a named wrapper and forwarding
  methods, even when it currently contains only one `Registry<K>`.
- Generic code over "anything queryable" is not available by default. If such
  code becomes necessary, we will add a narrow trait for that proven use case.
- Multi-index registries must be careful about observer ordering. If a
  registration touches several concrete registries, register/drop code must
  ensure subscribers do not observe incoherent intermediate state.
- There is no generic cascade/projection framework. Repeated lifecycle code may
  appear before we know whether it is truly identical.

## Alternatives considered

**Keep simple-name lookup inside `Registry<K>`.** This was the first measured
optimization: `Registry<K>` maintained an eager simple-name map alongside exact
providers. Rejected because not every registry has simple-name semantics, and
the generic primitive should not carry memory, policy, and API surface for a
lookup mode many registries do not support.

**Add `SimpleNamed` and `NamedRegistry<K>` to `beans-core`.** This removes the
simple-name map from exact-only registries, but it still makes simple-name
lookup look like a generic engine capability. It also leaks naming semantics
into `beans-core`. Rejected in favor of domain registry structs.

**Introduce `Queryable<K>` and `Subscribable<K>` traits now.** These traits may
eventually be useful, but today they hide more meaning than they remove. The
call sites usually need to know whether they are talking to JVM types, Java
symbols, methods, fields, or packages. Rejected until a concrete generic
algorithm requires them.

**Model this as projections.** "Projection" describes part of the lifecycle
problem, but it suggests a generic projection framework. For now these are just
separate observable registries coordinated by a domain registry. Rejected as a
premature concept.

**Build an observable relational database abstraction.** The registry layer may
eventually accumulate database-like properties, but jumping to that abstraction
now would obscure the small set of operations the engine actually needs.
Rejected until repeated concrete pressure proves the need.
