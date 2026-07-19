use crate::model::JvmSource;

pub trait JvmSearchScope {
    fn contains(&self, source: &JvmSource) -> bool;
}

/// No-op implementation that makes every source searchable.
pub struct AllSources;

impl JvmSearchScope for AllSources {
    fn contains(&self, _source: &JvmSource) -> bool {
        true
    }
}
