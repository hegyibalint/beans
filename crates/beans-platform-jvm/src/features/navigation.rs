use beans_core::features::navigation::{DefinitionProvider, DefinitionRequest, NavigationResult};

use crate::engine::JvmEngine;

use super::JvmFeatureContext;

impl<'request>
    DefinitionProvider<JvmFeatureContext<'_>, DefinitionRequest<'request>, NavigationResult>
    for JvmEngine
{
    fn goto_definition(
        &self,
        _context: &JvmFeatureContext<'_>,
        _request: &DefinitionRequest<'request>,
    ) -> Option<NavigationResult> {
        // Binary navigation requires source mapping, which is not wired yet.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_core::{
        classpath::Unrestricted, origin::Origin, query_scope::QueryScope, revision::Revision,
    };

    #[test]
    fn binaries_without_source_mapping_have_no_definition_location() {
        let engine = JvmEngine::default();
        let context = JvmFeatureContext {
            scope: QueryScope::new(Revision::default(), &Unrestricted),
        };

        assert!(
            engine
                .goto_definition(
                    &context,
                    &DefinitionRequest {
                        source: &Origin::class_file("Example.class"),
                        offset: 0,
                    },
                )
                .is_none()
        );
    }
}
