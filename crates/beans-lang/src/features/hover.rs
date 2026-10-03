use beans_lang_java::features::JavaFeatureContext;
use beans_platform_jvm::features::JvmFeatureContext;

use crate::Languages;

use super::{HoverProvider, HoverRequest, HoverResponse, LanguagesFeatureContext};

impl<'request> HoverProvider<LanguagesFeatureContext<'_>, HoverRequest<'request>, HoverResponse>
    for Languages
{
    fn hover(
        &self,
        context: &LanguagesFeatureContext<'_>,
        request: &HoverRequest<'request>,
    ) -> Option<HoverResponse> {
        let java_context = JavaFeatureContext {
            scope: context.scope,
            jvm: &self.jvm,
        };
        self.java_features()
            .hover(&java_context, request)
            .or_else(|| {
                self.jvm_features().hover(
                    &JvmFeatureContext {
                        scope: context.scope,
                    },
                    request,
                )
            })
    }
}
