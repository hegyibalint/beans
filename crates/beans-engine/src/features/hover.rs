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
            scope: QueryScope::new(&Unrestricted),
            workspace_root: Some(&self.workspace_root),
        };
        self.java.hover(&context, request)
    }
}
