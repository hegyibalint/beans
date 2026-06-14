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
}
