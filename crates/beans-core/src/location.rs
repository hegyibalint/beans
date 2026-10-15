use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location {
    pub entries: Vec<LocationEntry>,
}

impl Location {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::new(LocationEntry::File(path.into()))
    }

    pub fn new(entry: LocationEntry) -> Self {
        Self {
            entries: vec![entry],
        }
    }

    pub fn entry(&self, entry: LocationEntry) -> Self {
        let mut child = self.clone();
        child.entries.push(entry);
        child
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LocationEntry {
    File(PathBuf),
    Virtual(String),
    Container(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn file_constructor_preserves_the_path_as_the_first_entry() {
        let path = PathBuf::from("relative/../library.jar");
        let location = Location::file(path.clone());
        assert_eq!(location.entries, vec![LocationEntry::File(path)]);
        assert_eq!(
            Location::file("library.jar"),
            Location::file(PathBuf::from("library.jar"))
        );
    }

    #[test]
    fn identical_entries_in_different_roots_are_distinct() {
        let a = Location::file("/project/a.jar").entry(container("org/example/Foo.java"));
        let b = Location::file("/project/b.jar").entry(container("org/example/Foo.java"));
        assert_ne!(a, b);
        assert_eq!(HashSet::from([a.clone(), a, b]).len(), 2);
    }

    #[test]
    fn nesting_preserves_parent_identity_and_entry_boundaries() {
        let root = Location::file("/project/app.zip");
        let jar = root.entry(container("lib/a.jar"));
        let class = jar.entry(container("org/example/Foo.class"));
        assert_eq!(root.entries.len(), 1);
        assert_eq!(jar.entries.len(), 2);
        assert_eq!(
            class.entries,
            vec![
                LocationEntry::File("/project/app.zip".into()),
                container("lib/a.jar"),
                container("org/example/Foo.class"),
            ]
        );
        assert_ne!(
            class,
            root.entry(container("lib/a.jar/org/example/Foo.class"))
        );
        assert_ne!(
            class,
            root.entry(container("org/example/Foo.class"))
                .entry(container("lib/a.jar"))
        );
    }

    #[test]
    fn virtual_entries_preserve_the_complete_uri() {
        let uri = "BEANS-jvm:///library/My%20Type.class";
        let location = Location::new(LocationEntry::Virtual(uri.into()));
        assert_eq!(location.entries, vec![LocationEntry::Virtual(uri.into())]);
        assert_ne!(
            location,
            Location::new(LocationEntry::Virtual(
                "beans-jvm:///library/My%20Type.class".into()
            ))
        );
        assert_ne!(location, Location::file(uri));
    }

    #[test]
    fn entry_kinds_are_part_of_identity() {
        let value = "Example.java";
        let locations = [
            Location::new(LocationEntry::File(value.into())),
            Location::new(LocationEntry::Virtual(value.into())),
            Location::new(LocationEntry::Container(value.into())),
        ];
        assert_eq!(HashSet::from(locations).len(), 3);
    }

    fn container(name: &str) -> LocationEntry {
        LocationEntry::Container(name.into())
    }
}
