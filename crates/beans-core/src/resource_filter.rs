mod file;

pub use file::FileResourceFilter;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::location::Location;

/// A serializable filter over resource locations.
/// Membership does not resolve symbols or check language-level accessibility.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceFilter {
    File(FileResourceFilter),
}

impl ResourceFilter {
    pub fn file(root: impl AsRef<Path>) -> Self {
        Self::File(FileResourceFilter::new(root))
    }

    pub fn contains(&self, resource: &Location) -> bool {
        match self {
            Self::File(filter) => filter.contains(resource),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::LocationEntry;

    #[test]
    fn unrestricted_is_not_a_filter_entry() {
        assert!(
            serde_json::from_value::<ResourceFilter>(serde_json::json!("Unrestricted")).is_err()
        );
    }

    #[test]
    fn file_variants_delegate_membership() {
        let filter = ResourceFilter::file("/library.jar");
        let class = Location::file("/library.jar")
            .entry(LocationEntry::Container("p/Example.class".into()));
        assert!(filter.contains(&class));
        assert!(!filter.contains(&Location::file("/other/Example.class")));
    }

    #[test]
    fn serde_preserves_normalized_roots_and_entry_order() {
        let filters = vec![
            ResourceFilter::file("/libs/old/../b.jar"),
            ResourceFilter::file("/libs/a.jar"),
        ];
        let json = serde_json::to_value(&filters).unwrap();
        assert_eq!(
            json,
            serde_json::json!([
                {"File": {"root": "/libs/b.jar"}},
                {"File": {"root": "/libs/a.jar"}},
            ])
        );
        assert_eq!(
            serde_json::from_value::<Vec<ResourceFilter>>(json).unwrap(),
            filters
        );
    }
}
