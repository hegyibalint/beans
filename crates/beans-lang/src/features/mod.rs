//! Feature-local dispatch to the concretely owned language and platform engines.

use beans_core::{classpath::Classpath, query_scope::QueryScope};
use beans_lang_java::features::JavaFeatureContext;
use beans_platform_jvm::features::JvmFeatureContext;

pub use beans_core::features::{
    DefinitionProvider, DefinitionRequest, HoverProvider, HoverRequest, HoverResponse,
    LanguageFeatures, NavigationResult,
};

use crate::Languages;

mod hover;
mod navigation;

/// Binds feature requests to the aggregate's current revision and caller-provided visibility.
/// Constructed through [`Languages::feature_context`], without exposing engine state.
/// The aggregate cannot advance while a context is still in use:
///
/// ```compile_fail
/// use beans_core::classpath::Unrestricted;
/// use beans_lang::Languages;
///
/// let mut languages = Languages::default();
/// let context = languages.feature_context(&Unrestricted);
/// languages.process_document("untitled:Example.java", "java", "class Example {}");
/// let _ = context.scope().revision();
/// ```
#[derive(Clone, Copy)]
pub struct LanguagesFeatureContext<'a> {
    scope: QueryScope<'a>,
}

impl<'a> LanguagesFeatureContext<'a> {
    pub fn scope(&self) -> QueryScope<'a> {
        self.scope
    }
}

impl Languages {
    /// Captures the current revision and borrows visibility for feature requests.
    /// The returned context keeps this aggregate borrowed while requests use it.
    ///
    /// ```
    /// use beans_core::{classpath::Unrestricted, origin::Origin};
    /// use beans_lang::{HoverProvider, HoverRequest, Languages};
    ///
    /// let mut languages = Languages::default();
    /// let uri = "untitled:Example.java";
    /// let contents = "class C extends Target {}";
    /// languages.process_document(uri, "java", contents);
    /// let source = Origin::uri(uri);
    /// let context = languages.feature_context(&Unrestricted);
    /// let hover = languages.hover(&context, &HoverRequest {
    ///     source: &source,
    ///     contents,
    ///     offset: contents.find("Target").unwrap(),
    /// }).unwrap();
    /// assert_eq!(hover.contents(), "Target");
    /// ```
    pub fn feature_context<'a>(
        &'a self,
        classpath: &'a dyn Classpath,
    ) -> LanguagesFeatureContext<'a> {
        LanguagesFeatureContext {
            scope: QueryScope::new(self.revision, classpath),
        }
    }

    // Dispatch requires the complete contract, not just the feature being invoked.
    fn java_features(&self) -> &impl for<'a> LanguageFeatures<JavaFeatureContext<'a>> {
        &self.java
    }

    fn jvm_features(&self) -> &impl for<'a> LanguageFeatures<JvmFeatureContext<'a>> {
        &self.jvm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_core::{classpath::Unrestricted, revision::Revision};

    #[test]
    fn feature_context_binds_the_current_revision_and_borrows_visibility() {
        let mut languages = Languages::default();
        let classpath = Unrestricted;
        let initial = languages.feature_context(&classpath);
        assert_eq!(initial.scope().revision(), Revision::default());

        languages.process_document("untitled:Example.java", "java", "class Example {}");
        let current = languages.feature_context(&classpath);

        assert_eq!(current.scope().revision(), languages.revision());
        assert!(std::ptr::eq(
            current.scope().classpath(),
            &classpath as &dyn Classpath,
        ));
    }
}
