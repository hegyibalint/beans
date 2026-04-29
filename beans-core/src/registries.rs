//! Cross-layer registry aggregator.
//!
//! Per ADR-0012 each registry is its own typed slot; per ADR-0019 each
//! language module owns its own registries. [`Registries`] is the bag
//! that bundles every registry visible to a graph instance — JVM-projection
//! lookups and the per-language registries gated by their feature flags.
//! Resolution code names the registry it is querying (e.g.
//! `ctx.jvm.types.query(&key)`); there is no generic
//! `Registries::query(...)` entry point.
//!
//! Cloning a [`Registries`] clones each inner registry; per ADR-0015 each
//! [`Registry`](crate::graph::Registry) is internally `Rc<RefCell<_>>`,
//! so the clones share state. This is the intended pattern: every node
//! that needs to register receives a clone of [`Registries`] and runs
//! [`Registry::register`](crate::graph::Registry::register) against the
//! relevant slot.

use crate::jvm::registries::JvmRegistries;

#[cfg(feature = "java")]
use crate::languages::java::registries::JavaRegistries;

/// Cross-layer aggregator. Every graph instance owns one of these and
/// passes it as the `Ctx` for [`RegistryQuery`](crate::graph::RegistryQuery)
/// implementations.
#[derive(Clone, Default)]
pub struct Registries {
    pub jvm: JvmRegistries,

    #[cfg(feature = "java")]
    pub java: JavaRegistries,
}

impl Registries {
    pub fn new() -> Self {
        Self::default()
    }
}
