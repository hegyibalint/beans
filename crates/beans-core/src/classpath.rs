mod unrestricted;

pub use unrestricted::Unrestricted;

use crate::location::Location;

/// Determines which resources are available to a query.
/// Membership does not resolve symbols or check language-level accessibility.
pub trait Classpath {
    fn contains_source(&self, resource: &Location) -> bool;
    fn contains_class(&self, resource: &Location) -> bool;
}
