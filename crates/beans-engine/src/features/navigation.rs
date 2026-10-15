use beans_core::{
    classpath::Unrestricted,
    features::{DefinitionRequest, NavigationResult},
    query_scope::QueryScope,
    resource::ResourceId,
    revision::Revision,
};
use beans_lang_java::features::JavaFeatureContext;

use crate::{Engine, storage::ModelQuery};

impl Engine {
    pub fn source_content(
        &self,
        source: &ResourceId,
        revision: Revision,
    ) -> Result<Option<String>, String> {
        let mut reader = self.storage.reader()?;
        let query = ModelQuery::at_revision(&mut reader, revision)?;
        query
            .content(source)?
            .map(|content| String::from_utf8(content).map_err(|error| error.to_string()))
            .transpose()
    }

    pub fn feature_goto_definition(
        &self,
        request: &DefinitionRequest<'_>,
    ) -> Result<Option<NavigationResult>, String> {
        let mut reader = self.storage.reader()?;
        let models = ModelQuery::new(&mut reader)?;
        let context = JavaFeatureContext {
            models: &models,
            scope: QueryScope::new(models.revision(), &Unrestricted),
        };
        beans_lang_java::features::goto_definition(&context, request)
    }
}
