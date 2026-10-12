use beans_core::{
    classpath::Unrestricted,
    features::{DefinitionProvider, DefinitionRequest, NavigationResult},
    query_scope::QueryScope,
};
use beans_lang_java::features::JavaFeatureContext;

use crate::Engine;

impl Engine {
    pub fn feature_goto_definition(
        &self,
        request: &DefinitionRequest<'_>,
    ) -> Option<NavigationResult> {
        let context = JavaFeatureContext {
            scope: QueryScope::new(self.revision, &Unrestricted),
            jvm: &self.jvm,
        };
        self.java.goto_definition(&context, request)
    }
}
