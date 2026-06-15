//! The composed registry bag.
//!
//! Each vertical owns its registry struct; the facade composes them.
//! Per ADR-0012 access is by named field — `registries.jvm.types`,
//! `registries.java.symbols` — with no generic dispatch. The `jvm`
//! field is the shared cross-vertical surface; each vertical contributes
//! one per-language field. Every vertical is composed unconditionally
//! (ADR-0033); a new language adds a field here alongside its crate.

use beans_lang_java::JavaRegistries;
use beans_lang_jvm::JvmRegistries;

#[derive(Default)]
pub struct Registries {
    pub jvm: JvmRegistries,
    pub java: JavaRegistries,
}

impl Registries {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a notification batch across every composed registry — the
    /// JVM projection surface and each language vertical. `Workspace`
    /// wraps bulk and incremental indexing in this so a batch of
    /// integrations emits each changed key's subscriber callbacks once,
    /// at [`Self::commit_batch`], instead of churning per node.
    pub fn begin_batch(&self) {
        self.jvm.begin_batch();
        self.java.begin_batch();
    }

    /// Close the notification batch opened by [`Self::begin_batch`].
    ///
    /// Each child `Registry<K>` snapshots and fires as its own observer
    /// boundary. The composed bag coordinates fields, but it does not
    /// provide an atomic all-registries callback snapshot.
    pub fn commit_batch(&self) {
        self.jvm.commit_batch();
        self.java.commit_batch();
    }
}
