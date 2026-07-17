use crate::model::JvmSource;

pub trait JvmScope {
    fn in_scope(&self, source: &JvmSource) -> bool;
}

/// No-op implementation of JvmScope that considers all sources to be in scope.
pub struct AllSources;

impl JvmScope for AllSources {
    fn in_scope(&self, _source: &JvmSource) -> bool {
        true
    }
}
