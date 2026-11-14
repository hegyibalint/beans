use beans_core::features::{HoverProvider, HoverRequest, HoverResponse};
use beans_lang_java::features::JavaFeatureContext;

use crate::{Engine, location_matches::matcher, modules::load_modules};

impl Engine {
    pub fn feature_hover(
        &self,
        request: &HoverRequest<'_>,
    ) -> Result<Option<HoverResponse>, String> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let modules = load_modules(&transaction)?;
        let match_locations = matcher(&modules);
        let context = JavaFeatureContext {
            connection: &transaction,
            match_locations: Some(&match_locations),
            workspace_root: Some(&self.workspace_root),
        };
        self.java.hover(&context, request)
    }
}
