//! Cross-layer registry aggregator.
//!
//! Per ADR-0012 each registry is its own typed slot. [`Registries`] is the
//! flat bag that bundles every registry visible to a [`crate::Beans`]
//! instance — JVM-projection lookups and the per-language registries
//! gated by their feature flags. Resolution code names the registry it
//! is querying directly (e.g. `beans.registries.jvm_types.providers(&key)`);
//! there is no generic `Registries::query(...)` entry point.
//!
//! Per ADR-0019 each language *module* still owns the typed key for its
//! own-language registry (e.g. [`crate::languages::java::JavaSymbolKey`]).
//! What flattens here is the bag — there's no `JvmRegistries` /
//! `JavaRegistries` middle struct any more, just direct named fields.
//! Adding a new language registry adds one field on `Registries` (gated
//! by the language's feature) and one variant in the future
//! `Registration` enum used for cleanup.
//!
//! Not [`Clone`]. Per workspace there is exactly one [`Registries`],
//! owned by [`crate::Beans`]. Code that needs read access threads
//! `&beans.registries` through the call chain. The internal
//! [`Registry<K>`](crate::registry::Registry) is `Rc<RefCell<...>>` for
//! re-entrant subscription support — that sharing is *internal* to a
//! single registry and doesn't bleed up to the bag.

use crate::jvm::{JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey};
use crate::registry::Registry;

#[cfg(feature = "java")]
use crate::languages::java::JavaSymbolKey;

/// Cross-layer aggregator. Every [`crate::Beans`] instance owns one.
/// Resolution code accesses each typed registry by its named field
/// directly; per ADR-0012 there is no generic dispatch.
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
