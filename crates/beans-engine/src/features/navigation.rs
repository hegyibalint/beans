use beans_core::features::{DefinitionProvider, DefinitionRequest, NavigationResult};
use beans_lang_java::features::JavaFeatureContext;

use crate::{Engine, scopes::load_scopes};

impl Engine {
    pub fn feature_goto_definition(
        &self,
        request: &DefinitionRequest<'_>,
    ) -> Result<Option<NavigationResult>, String> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let scopes = load_scopes(&transaction)?;
        let context = JavaFeatureContext {
            connection: &transaction,
            scopes: &scopes,
            workspace_root: Some(&self.workspace_root),
        };
        self.java.goto_definition(&context, request)
    }
}
