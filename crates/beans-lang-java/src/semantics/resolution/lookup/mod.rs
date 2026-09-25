mod external;
mod internal;

pub(super) use external::lookup_external;
pub(super) use internal::{lookup_downward, lookup_upward, lookup_upward_from_type_header};
