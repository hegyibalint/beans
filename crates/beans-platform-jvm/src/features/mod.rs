//! JVM implementations of protocol-neutral editor features.

use beans_core::engine::QueryScope;

mod hover;
mod navigation;

/// Binds platform feature queries to model versions and source visibility.
#[derive(Clone, Copy)]
pub struct JvmFeatureContext<'a> {
    pub scope: QueryScope<'a>,
}
