use beans_core::{
    features::{DefinitionProvider, DefinitionRequest, NavigationResult},
    query_scope::QueryScope,
};
use beans_lang_java::features::JavaFeatureContext;

use crate::Engine;

impl Engine {
    pub fn feature_goto_definition(
        &self,
        request: &DefinitionRequest<'_>,
    ) -> Result<Option<NavigationResult>, String> {
        let transaction = self
            .storage
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        let context = JavaFeatureContext {
            connection: &transaction,
            scope: QueryScope::unrestricted(),
            workspace_root: Some(&self.workspace_root),
        };
        self.java.goto_definition(&context, request)
    }
}
