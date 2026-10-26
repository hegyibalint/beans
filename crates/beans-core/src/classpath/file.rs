use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    location::{Location, LocationEntry},
    paths::normalize,
};

/// A filesystem root, admitting filesystem descendants and direct container contents.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileClasspath {
    root: PathBuf,
}

impl FileClasspath {
    /// The root must be absolute, but need not exist.
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        assert!(root.is_absolute(), "classpath roots must be absolute");
        Self {
            root: normalize(root),
        }
    }

    pub fn contains_source(&self, resource: &Location) -> bool {
        let [LocationEntry::File(path)] = resource.entries.as_slice() else {
            return false;
        };
        path.is_absolute() && normalize(path).starts_with(&self.root)
    }

    pub fn contains_class(&self, resource: &Location) -> bool {
        match resource.entries.as_slice() {
            [LocationEntry::File(_)] => self.contains_source(resource),
            [LocationEntry::File(path), LocationEntry::Container(_)] => {
                path.is_absolute() && normalize(path) == self.root
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_files_match_without_admitting_siblings() {
        let classpath = FileClasspath::new("/classes/p/C.class");
        let resource = Location::file("/classes/p/C.class");
        assert!(classpath.contains_source(&resource));
        assert!(classpath.contains_class(&resource));

        for path in ["/classes/p/D.class", "/classes/p/C.class.bak"] {
            let resource = Location::file(path);
            assert!(!classpath.contains_source(&resource));
            assert!(!classpath.contains_class(&resource));
        }
    }

    #[test]
    fn directories_match_descendants_by_path_components() {
        let classpath = FileClasspath::new("/classes");
        for path in [
            "/classes/C.class",
            "/classes/p/q/C.class",
            "/classes/p/C.java",
        ] {
            let resource = Location::file(path);
            assert!(classpath.contains_source(&resource));
            assert!(classpath.contains_class(&resource));
        }
        for path in [
            "/classes-other/C.class",
            "/other/C.class",
            "classes/C.class",
        ] {
            let resource = Location::file(path);
            assert!(!classpath.contains_source(&resource));
            assert!(!classpath.contains_class(&resource));
        }
    }

    #[test]
    fn containers_admit_only_direct_binary_entries_from_the_exact_root() {
        let classpath = FileClasspath::new("/libs/example.jar");
        for name in ["C.class", "p/q/C.class"] {
            let resource =
                Location::file("/libs/example.jar").entry(LocationEntry::Container(name.into()));
            assert!(!classpath.contains_source(&resource));
            assert!(classpath.contains_class(&resource));
        }
        for path in [
            "/libs/other.jar",
            "/libs/example.jar.bak",
            "libs/example.jar",
        ] {
            let resource = Location::file(path).entry(LocationEntry::Container("p/C.class".into()));
            assert!(!classpath.contains_source(&resource));
            assert!(!classpath.contains_class(&resource));
        }
    }

    #[test]
    fn directories_do_not_expose_contained_archives() {
        let classpath = FileClasspath::new("/classes");
        let resource =
            Location::file("/classes/lib.jar").entry(LocationEntry::Container("p/C.class".into()));
        assert!(!classpath.contains_source(&resource));
        assert!(!classpath.contains_class(&resource));
    }

    #[test]
    fn unsupported_location_shapes_are_rejected() {
        let classpath = FileClasspath::new("/libs/example.jar");
        let root = Location::file("/libs/example.jar");
        for resource in [
            Location { entries: vec![] },
            Location::new(LocationEntry::Virtual("file:///libs/example.jar".into())),
            Location::new(LocationEntry::Container("/libs/example.jar".into())),
            root.entry(LocationEntry::File("p/C.class".into())),
            root.entry(LocationEntry::Virtual("untitled:C.class".into())),
            root.entry(LocationEntry::Container("lib/nested.jar".into()))
                .entry(LocationEntry::Container("p/C.class".into())),
        ] {
            assert!(!classpath.contains_source(&resource));
            assert!(!classpath.contains_class(&resource));
        }
    }

    #[test]
    fn directory_roots_and_resources_are_lexically_normalized() {
        let classpath = FileClasspath::new("/project/out/../classes/.");
        let resource = Location::file("/project/classes/p/../C.class");
        assert!(classpath.contains_source(&resource));
        assert!(classpath.contains_class(&resource));

        let outside = Location::file("/project/classes/../outside/C.class");
        assert!(!classpath.contains_source(&outside));
        assert!(!classpath.contains_class(&outside));
    }

    #[test]
    fn container_roots_and_outer_paths_are_lexically_normalized() {
        let classpath = FileClasspath::new("/libs/old/../example.jar");
        let resource = Location::file("/libs/./example.jar")
            .entry(LocationEntry::Container("p/C.class".into()));
        assert!(classpath.contains_class(&resource));
    }

    #[test]
    #[should_panic(expected = "classpath roots must be absolute")]
    fn relative_roots_are_rejected() {
        FileClasspath::new("classes");
    }
}
