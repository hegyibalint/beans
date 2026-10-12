//! Java implementations of protocol-neutral editor features.

use beans_core::query_scope::QueryScope;
use beans_platform_jvm::JvmPlatform;

use crate::model_query::JavaModelQuery;

mod hover;
mod navigation;

/// Model access and legacy discovery must use the same selected view.
#[derive(Clone, Copy)]
pub struct JavaFeatureContext<'a, Q: JavaModelQuery> {
    pub models: &'a Q,
    pub scope: QueryScope<'a>,
    pub jvm: &'a JvmPlatform,
}
