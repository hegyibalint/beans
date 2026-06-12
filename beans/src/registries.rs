//! The composed registry bag.
//!
//! Each vertical owns its registry struct; the facade composes them.
//! Per ADR-0012 access is by named field — `registries.jvm.types`,
//! `registries.java.symbols` — with no generic dispatch. The `jvm`
//! field is the shared cross-vertical surface; per-language fields are
//! gated by their feature.

use beans_lang_jvm::JvmRegistries;

#[cfg(feature = "java")]
use beans_lang_java::JavaRegistries;

#[derive(Default)]
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
