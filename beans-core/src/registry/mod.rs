//! The registry layer — typed-key indices over graph nodes, plus the
//! query abstractions that compose them.
//!
//! This is one of the engine's two pillars (the other is [`graph`]).
//! Graph owns nodes; registries index them. The two are orthogonal:
//! graph has zero knowledge of registries, registries depend on graph
//! only for `NodeId` and the `NodeHandle` marker trait.
//!
//! Module layout:
//!
//! - [`single`] — [`Registry<K>`], the single-key typed multi-provider
//!   primitive. Per ADR-0013 each key maps to a list of providers; per
//!   ADR-0014 RAII handles ([`ProviderHandle`], [`SubscriptionHandle`])
//!   tie registration lifetime to node lifetime; per ADR-0015 the
//!   inner is `Rc<RefCell<...>>` for re-entrant subscription support.
//!   Per ADR-0008 `register` and the provider drop path auto-fire
//!   subscribers — no manual notify required for normal mutations.
//! - [`query`] — the cross-registry abstractions: [`Queryable<M>`]
//!   trait + [`QueryResult`] tri-state for one-shot queries, plus
//!   [`MultiQuery`] for stored, subscription-backed cached queries
//!   over a heterogeneous list of [`RegistryQuery`] variants.
//!
//! [`Registries`] (the bag below) lives at this module's top level
//! because it's the cross-layer aggregator every [`crate::Beans`]
//! instance owns. Per ADR-0019 each language registry is a feature-
//! gated field; the bag is flat (no per-language wrapper structs).
//! Not [`Clone`]: per workspace there is exactly one.
//!
//! [`graph`]: crate::graph

pub mod query;
pub mod single;

pub use query::{
    all_matches, first_match, ByFqn, MultiQuery, MultiSubscriptionHandle, Queryable, QueryResult,
    RegistryQuery,
};
pub use single::{Callback, ProviderHandle, Registry, SubscriptionHandle, SubscriptionId};

use crate::jvm::{JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey};

#[cfg(feature = "java")]
use crate::languages::java::JavaSymbolKey;

/// Cross-layer registry aggregator. Every [`crate::Beans`] instance
/// owns one. Resolution code accesses each typed registry by its named
/// field directly; per ADR-0012 there is no generic dispatch.
#[derive(Default)]
pub struct Registries {
    pub jvm_types: Registry<JvmTypeKey>,
    pub jvm_methods: Registry<JvmMethodKey>,
    pub jvm_fields: Registry<JvmFieldKey>,
    pub jvm_constructors: Registry<JvmConstructorKey>,
    pub jvm_packages: Registry<PackageKey>,

    #[cfg(feature = "java")]
    pub java_symbols: Registry<JavaSymbolKey>,
}

impl Registries {
    pub fn new() -> Self {
        Self::default()
    }
}
