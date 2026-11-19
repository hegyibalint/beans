use beans_core::{
    features::{HoverProvider, HoverRequest, HoverResponse},
    location_match::LocationMatcher,
};
use beans_lang_java::features::JavaFeatureContext;

use crate::{Engine, modules::load_modules};

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
        let location_matcher = LocationMatcher::new(&modules);
        let context = JavaFeatureContext {
            connection: &transaction,
            location_matcher: Some(&location_matcher),
            workspace_root: Some(&self.workspace_root),
        };
        self.java.hover(&context, request)
    }
}
