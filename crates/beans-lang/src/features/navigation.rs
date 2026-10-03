use beans_lang_java::features::JavaFeatureContext;
use beans_platform_jvm::features::JvmFeatureContext;

use crate::Languages;

use super::{DefinitionProvider, DefinitionRequest, LanguagesFeatureContext, NavigationResult};

impl<'request>
    DefinitionProvider<LanguagesFeatureContext<'_>, DefinitionRequest<'request>, NavigationResult>
    for Languages
{
    fn goto_definition(
        &self,
        context: &LanguagesFeatureContext<'_>,
        request: &DefinitionRequest<'request>,
    ) -> Option<NavigationResult> {
        let java_context = JavaFeatureContext {
            scope: context.scope,
            jvm: &self.jvm,
        };
        self.java_features()
            .goto_definition(&java_context, request)
            .or_else(|| {
                self.jvm_features().goto_definition(
                    &JvmFeatureContext {
                        scope: context.scope,
                    },
                    request,
                )
            })
    }
}
