use beans_core::features::hover::{HoverProvider, HoverRequest, HoverResponse};

use crate::engine::JvmEngine;

use super::JvmFeatureContext;

impl<'request> HoverProvider<JvmFeatureContext<'_>, HoverRequest<'request>, HoverResponse>
    for JvmEngine
{
    fn hover(
        &self,
        _context: &JvmFeatureContext<'_>,
        _request: &HoverRequest<'request>,
    ) -> Option<HoverResponse> {
        // Origin positions are not modeled; class-file debug attributes are optional
        // and do not supply symbol byte ranges (JVMS §4.7.10 and §4.7.12).
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
    fn unmodeled_source_positions_have_no_hover() {
        let engine = JvmEngine::default();
        let context = JvmFeatureContext {
            scope: QueryScope::new(Revision::default(), &Unrestricted),
        };
        let source = Origin::class_file("Example.class");
        let request = HoverRequest {
            source: &source,
            contents: "",
            offset: 0,
        };

        assert!(engine.hover(&context, &request).is_none());
    }
}
