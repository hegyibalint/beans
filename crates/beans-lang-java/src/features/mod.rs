//! Java implementations of protocol-neutral editor features.

use beans_core::query_scope::QueryScope;
use beans_platform_jvm::engine::JvmEngine;

mod hover;
mod navigation;

/// Binds Java feature queries to a scope and the platform supplying JVM candidates.
#[derive(Clone, Copy)]
pub struct JavaFeatureContext<'a> {
    pub scope: QueryScope<'a>,
    pub jvm: &'a JvmEngine,
}
