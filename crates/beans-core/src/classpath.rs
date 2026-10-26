mod file;

pub use file::FileClasspath;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::location::Location;

/// Determines which resources are available to a query.
/// Membership does not resolve symbols or check language-level accessibility.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classpath {
    File(FileClasspath),
    Unrestricted,
}

impl Classpath {
    pub fn file(root: impl AsRef<Path>) -> Self {
        Self::File(FileClasspath::new(root))
    }

    pub fn contains_source(&self, resource: &Location) -> bool {
        match self {
            Self::File(classpath) => classpath.contains_source(resource),
            Self::Unrestricted => true,
        }
    }

    pub fn contains_class(&self, resource: &Location) -> bool {
        match self {
            Self::File(classpath) => classpath.contains_class(resource),
            Self::Unrestricted => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::LocationEntry;

    #[test]
    fn unrestricted_includes_files_virtual_resources_and_nested_entries() {
        let classpath = Classpath::Unrestricted;
        let file = Location::file("/src/Example.java");
        let virtual_resource =
            Location::new(LocationEntry::Virtual("untitled:Example.java".into()));
        let class = Location::file("/library.jar")
            .entry(LocationEntry::Container("p/Example.class".into()));

        for resource in [file, virtual_resource, class] {
            assert!(classpath.contains_source(&resource));
            assert!(classpath.contains_class(&resource));
        }
    }

    #[test]
    fn file_variants_delegate_both_membership_checks() {
        let classpath = Classpath::file("/library.jar");
        let class = Location::file("/library.jar")
            .entry(LocationEntry::Container("p/Example.class".into()));
        assert!(!classpath.contains_source(&class));
        assert!(classpath.contains_class(&class));
        assert!(!classpath.contains_class(&Location::file("/other/Example.class")));
    }

    #[test]
    fn serde_preserves_variants_normalized_roots_and_entry_order() {
        let classpaths = vec![
            Classpath::file("/libs/old/../b.jar"),
            Classpath::Unrestricted,
            Classpath::file("/libs/a.jar"),
        ];
        let json = serde_json::to_value(&classpaths).unwrap();
        assert_eq!(
            json,
            serde_json::json!([
                {"File": {"root": "/libs/b.jar"}},
                "Unrestricted",
                {"File": {"root": "/libs/a.jar"}},
            ])
        );
        assert_eq!(
            serde_json::from_value::<Vec<Classpath>>(json).unwrap(),
            classpaths
        );
    }
}
