//! The Java vertical's registry bag.
//!
//! Java-side declarations register here under their source FQN. The
//! shared JVM projections register in
//! `beans_lang_jvm::JvmRegistries` — the cross-vertical surface. The
//! `beans` facade composes both into its `Registries`.

use beans_core::registry::Registry;

use crate::model::keys::JavaSymbolKey;

#[derive(Default)]
pub struct JavaRegistries {
    pub symbols: Registry<JavaSymbolKey>,
}

impl JavaRegistries {
    pub fn new() -> Self {
        Self::default()
    }
}
