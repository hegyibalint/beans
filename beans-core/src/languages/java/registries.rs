//! Java-side registry bag.
//!
//! Per ADR-0012 / ADR-0019 each language module owns its own registries.
//! Java has only one Java-side registry today — keyed by [`JavaSymbolKey`]
//! — but the bundle struct exists so resolution code can name
//! `registries.java.symbols` symmetrically with `registries.jvm.types`,
//! and so additional Java-specific registries (e.g., a future
//! `JavaPermitsKey` if the rules diverge) land without churning the
//! consumer-facing surface.

use crate::registry::Registry;
use crate::languages::java::keys::JavaSymbolKey;

/// All Java-side registries.
#[derive(Clone, Default)]
pub struct JavaRegistries {
    pub symbols: Registry<JavaSymbolKey>,
}

impl JavaRegistries {
    pub fn new() -> Self {
        Self::default()
    }
}
