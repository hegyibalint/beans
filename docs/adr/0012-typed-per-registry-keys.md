# ADR-0012: Use typed per-registry keys, not a shared key enum

## Status

Accepted

## Context

The semantic graph is fronted by a set of registries — one per "lookup
shape" the system needs (see [ARCHITECTURE.md](../../ARCHITECTURE.md)). The JVM registry is
keyed by an FQN plus method/field disambiguation. Kotlin has separate
registries for plain symbols, extension functions, and companions. Scala
adds implicits and extensions. Each registry has its own natural key.

A single shared key type — for example, an `enum RegistryKey { Jvm(..),
Kotlin(..), KotlinExtension(..), ... }` threaded through one big
`Registry<RegistryKey, NodeId>` — would let resolution code, subscriber
plumbing, and serialization use one type everywhere. It is the obvious
shape if you are coming from a generic platform mindset (ADR-0001), and
it matches how IntelliJ stubs its indexes.

The shared-enum shape pushes everything to runtime. Every lookup is
"build a key, dispatch on the variant inside the registry, and hope you
built the right variant." Mismatches between query and registry — asking
the JVM registry for a Kotlin extension key — are not compile errors;
they are silent misses that surface as broken resolution at the LSP
boundary. The blast radius of a wrong key in resolution code is large
and the feedback is slow.

## Decision

Each registry owns a **typed key struct** specific to its lookup shape.
Examples:

```rust
struct JvmMethodKey { owner: ClassFqn, name: String, params: Vec<JvmDescriptor> }
struct KotlinExtensionKey { receiver: TypeRef, name: String }
struct ScalaImplicitKey { target: TypeRef }
```

The `Registries` struct holds these as named typed fields, not as a map:

```rust
struct Registries {
    jvm_methods: Registry<JvmMethodKey>,
    jvm_fields: Registry<JvmFieldKey>,
    kotlin_extensions: Registry<KotlinExtensionKey>,
    // ...
}
```

A query that intends to hit the Kotlin extension registry must construct
a `KotlinExtensionKey` and call `registries.kotlin_extensions.query(...)`.
There is no path where a JVM method key reaches the Kotlin extension
registry: the types do not match, and the compiler refuses.

There is no shared `RegistryKey` enum and no generic
`Registries::query(key)` entry point. Resolution code names the registry
it is talking to, by field name.

## Consequences

**Positive.**

- Wrong-registry queries fail at compile time. This is a mistake we
  expect to make often during the cross-language phase — the compiler
  catches it for us instead of the integration tests.
- Each registry's key carries exactly the fields its lookup needs. No
  optional fields that mean "irrelevant for this variant," no
  `Option<JvmDescriptor>` that is `Some` for methods and `None` for
  fields.
- Reading resolution code is straightforward: the field name on
  `Registries` tells you which registry is being consulted.
- Adding a new registry (e.g., Groovy delegate) is a local change — a
  new key struct and a new field on `Registries`. Existing registries
  are untouched.

**Negative.**

- There is no generic "iterate over all registries" loop. If we ever
  need one (for diagnostics, GC checks, snapshot dumps), we have to
  write it by hand or reach for a macro. We are betting we will not
  need it often.
- Code that wants to be polymorphic across registries (rare) cannot be
  written generically without more machinery. The expected pattern is
  that each registry is consulted by name in resolution code that knows
  which language it is resolving.
- The `Registries` struct grows by one field per registry. With ~12
  registries across all five languages, the struct is wide but flat.
  This is fine; it is configuration, not data.

## Alternatives considered

**Single shared `RegistryKey` enum across all registries.** One enum
with a variant per registry, threaded through `Registry<RegistryKey>`
and a single `Registries::query(key)` method. Rejected because it pushes
all type discrimination to runtime. A typo in resolution code that
constructs the wrong variant becomes a silent miss — no panic, no
warning, just nothing in the result. The whole system relies on
registries returning the right thing for the right query, and we want
the type system to check that for us.

**`HashMap<TypeId, Box<dyn Any>>` registry of registries.** Same
problem as the shared enum, with extra downcasting. Rejected for the
same reasons plus the cost of `Box<dyn Any>`.

**Trait-object registry: `Box<dyn Registry>` with associated key type
hidden behind a trait method.** Compiles, but each call site has to
either statically know the concrete type (in which case the trait
object adds nothing) or work entirely through the trait (in which case
the key is type-erased and we are back to runtime dispatch). Rejected
as a worse version of the same idea.
