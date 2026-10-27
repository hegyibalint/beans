use beans_core::{
    compilation_scope::CompilationScopes,
    features::{HoverProvider, HoverRequest, HoverResponse},
};
use beans_lang_java::features::JavaFeatureContext;

use crate::Engine;

impl Engine {
    pub fn feature_hover(
        &self,
        request: &HoverRequest<'_>,
    ) -> Result<Option<HoverResponse>, String> {
        let transaction = self
            .storage
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        let context = JavaFeatureContext {
            connection: &transaction,
            scopes: &CompilationScopes::unrestricted(),
            workspace_root: Some(&self.workspace_root),
        };
        self.java.hover(&context, request)
    }
}
