use beans_core::{
    classpath::Unrestricted,
    features::{HoverRequest, HoverResponse},
    query_scope::QueryScope,
};
use beans_lang_java::features::JavaFeatureContext;

use crate::{Engine, storage::ModelQuery};

impl Engine {
    pub fn feature_hover(
        &self,
        request: &HoverRequest<'_>,
    ) -> Result<Option<HoverResponse>, String> {
        let mut reader = self.storage.reader()?;
        let models = ModelQuery::new(&mut reader)?;
        let context = JavaFeatureContext {
            models: &models,
            scope: QueryScope::new(models.revision(), &Unrestricted),
        };
        let response = beans_lang_java::features::hover(&context, request)?;
        if response.is_some()
            && models.content(request.source)?.as_deref() != Some(request.contents.as_bytes())
        {
            return Err("hover contents do not match the stored model".into());
        }
        Ok(response)
    }
}
