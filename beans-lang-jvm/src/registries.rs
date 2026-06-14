//! The shared JVM registry bag.
//!
//! Per ADR-0012 each registry has a typed key and is accessed by named
//! field — no generic dispatch. Per the vertical layout, this bag is
//! the *only* registry surface shared across language verticals: a
//! vertical's own registries live in its own crate
//! (`JavaRegistries`, ...), and the `beans` facade composes them all.

use beans_core::registry::Registry;

use crate::model::keys::{JvmConstructorKey, JvmFieldKey, JvmMethodKey, JvmTypeKey, PackageKey};

/// Registries for the JVM projection layer. Populated by every
/// vertical (each language registers its projections) and by bytecode
/// loading (jmod/JAR readers register here directly).
#[derive(Default)]
pub struct JvmRegistries {
    pub types: Registry<JvmTypeKey>,
    pub methods: Registry<JvmMethodKey>,
    pub fields: Registry<JvmFieldKey>,
    pub constructors: Registry<JvmConstructorKey>,
    pub packages: Registry<PackageKey>,
}

impl JvmRegistries {
    pub fn new() -> Self {
        Self::default()
    }
}
