use crate::{location::Location, source::SourceSpan};

/// Matches candidate spans within the requesting resource's compilation environment.
/// Each returned record is one admitted route; an empty result admits none.
pub type LocationMatcher<'a> =
    dyn Fn(&Location, &[SourceSpan]) -> Result<Vec<LocationMatch>, String> + 'a;

/// A source span and the artifact through which it is reachable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationMatch {
    /// The declaration's source span, independent of its reachability route.
    pub origin: SourceSpan,
    /// The artifact admitting access, not necessarily where the declaration was read.
    /// `None` means no intermediate artifact was needed.
    pub via: Option<Location>,
}
