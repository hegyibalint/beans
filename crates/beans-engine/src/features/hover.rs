use beans_core::{
    classpath::Unrestricted,
    features::{HoverProvider, HoverRequest, HoverResponse},
    query_scope::QueryScope,
};
use beans_lang_java::features::JavaFeatureContext;

use crate::Engine;

#[cfg(test)]
mod tests;

impl Engine {
    pub fn feature_hover(&self, request: &HoverRequest<'_>) -> Option<HoverResponse> {
        let context = JavaFeatureContext {
            scope: QueryScope::new(self.revision, &Unrestricted),
            jvm: &self.jvm,
        };
        self.java.hover(&context, request)
    }
}
