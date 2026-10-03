//! Protocol-neutral feature contracts and their request and response models.

pub mod hover;
pub mod navigation;

pub use hover::{HoverProvider, HoverRequest, HoverResponse};
pub use navigation::{DefinitionProvider, DefinitionRequest, NavigationResult};

/// Every integrated vertical implements all feature contracts for its own context,
/// including features that currently return no answer. Semantic integration fixes
/// the request/response types to core models, independently of protocol adapters.
/// Request borrows may have any lifetime, independently of the context's lifetime.
///
/// Implementing only one capability does not satisfy the integration contract:
///
/// ```compile_fail
/// use beans_core::features::{HoverProvider, HoverRequest, HoverResponse, LanguageFeatures};
///
/// struct HoverOnly;
/// impl<'a> HoverProvider<(), HoverRequest<'a>, HoverResponse> for HoverOnly {
///     fn hover(&self, _: &(), _: &HoverRequest<'a>) -> Option<HoverResponse> {
///         None
///     }
/// }
///
/// fn register(provider: &impl LanguageFeatures<()>) {}
/// register(&HoverOnly);
/// ```
///
/// Implementing both capabilities with other response types is not sufficient:
///
/// ```compile_fail
/// use beans_core::features::{
///     DefinitionProvider, DefinitionRequest, HoverProvider, HoverRequest, LanguageFeatures,
/// };
///
/// struct OtherResponses;
/// impl<'a> HoverProvider<(), HoverRequest<'a>, String> for OtherResponses {
///     fn hover(&self, _: &(), _: &HoverRequest<'a>) -> Option<String> { None }
/// }
/// impl<'a> DefinitionProvider<(), DefinitionRequest<'a>, String> for OtherResponses {
///     fn goto_definition(&self, _: &(), _: &DefinitionRequest<'a>) -> Option<String> { None }
/// }
///
/// fn register(provider: &impl LanguageFeatures<()>) {}
/// register(&OtherResponses);
/// ```
pub trait LanguageFeatures<C: ?Sized>:
    for<'request> HoverProvider<C, HoverRequest<'request>, HoverResponse>
    + for<'request> DefinitionProvider<C, DefinitionRequest<'request>, NavigationResult>
{
}

impl<T: ?Sized, C: ?Sized> LanguageFeatures<C> for T where
    T: for<'request> HoverProvider<C, HoverRequest<'request>, HoverResponse>
        + for<'request> DefinitionProvider<C, DefinitionRequest<'request>, NavigationResult>
{
}
