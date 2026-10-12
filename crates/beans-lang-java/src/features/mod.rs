//! Java implementations of protocol-neutral editor features.

use beans_core::query_scope::QueryScope;

use crate::model_query::JavaModelQuery;

pub use hover::hover;
pub use navigation::goto_definition;

mod hover;
mod navigation;

#[derive(Clone, Copy)]
pub struct JavaFeatureContext<'a, Q: JavaModelQuery> {
    pub models: &'a Q,
    pub scope: QueryScope<'a>,
}
