use beans_core::{
    classpath::Unrestricted,
    features::{HoverRequest, HoverResponse},
    query_scope::QueryScope,
};
use beans_lang_java::features::JavaFeatureContext;

use crate::{Engine, model_query::ModelQuery};

#[cfg(test)]
mod tests;

impl Engine {
    pub fn feature_hover(&self, request: &HoverRequest<'_>) -> Option<HoverResponse> {
        let models = ModelQuery::new(&self.storage);
        let context = JavaFeatureContext {
            models: &models,
            scope: QueryScope::new(self.revision, &Unrestricted),
        };
        beans_lang_java::features::hover(&context, request)
    }
}
