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

    /// Open a notification batch on every Java registry. Mutations stay
    /// immediate; subscriber callbacks defer to the matching
    /// [`Self::commit_batch`].
    pub fn begin_batch(&self) {
        self.symbols.begin_batch();
    }

    /// Close the notification batch on every Java registry.
    pub fn commit_batch(&self) {
        self.symbols.commit_batch();
    }
}
