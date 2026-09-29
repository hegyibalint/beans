//! Editor-facing queries over the current engine state, without protocol coordinates.

use beans_core::{
    engine::QueryEnvironment,
    model::{
        lsp::features::hover::{HoverProvider, HoverRequest, HoverResponse},
        source::{NavigationResult, Source},
    },
};

use crate::Engine;

/// A borrowed view of the engine for language features; no state is duplicated.
pub struct LanguageFeatures<'a> {
    engine: &'a Engine,
}

impl<'a> LanguageFeatures<'a> {
    pub(crate) fn new(engine: &'a Engine) -> Self {
        Self { engine }
    }

    /// Asks each vertical for hover content at the current revision, in priority order.
    pub fn hover(&self, request: &HoverRequest<'_>) -> Option<HoverResponse> {
        let providers: [&dyn HoverProvider; 2] = [&self.engine.java, &self.engine.jvm];
        providers
            .into_iter()
            .find_map(|provider| provider.hover(self.engine.revision, request))
    }

    pub fn goto_definition(&self, source: &Source, offset: usize) -> Option<NavigationResult> {
        self.engine.java.goto_definition(
            QueryEnvironment::new(self.engine.revision, self.engine.classpath.as_ref()),
            source,
            offset,
            &self.engine.jvm,
        )
    }
}
