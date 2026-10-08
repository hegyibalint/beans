use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceRoot {
    File { path: PathBuf },
    Virtual { provider: String, key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceEntry(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId {
    pub root: ResourceRoot,
    pub entries: Vec<ResourceEntry>,
}

impl ResourceId {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::new(ResourceRoot::File { path: path.into() })
    }

    pub fn new(root: ResourceRoot) -> Self {
        Self {
            root,
            entries: Vec::new(),
        }
    }

    pub fn entry(&self, entry: ResourceEntry) -> Self {
        let mut child = self.clone();
        child.entries.push(entry);
        child
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn file(path: &str) -> ResourceId {
        ResourceId::file(path)
    }

    #[test]
    fn file_constructor_preserves_the_path_and_has_no_entries() {
        let path = PathBuf::from("relative/../library.jar");
        let id = ResourceId::file(path.clone());
        assert_eq!(id.root, ResourceRoot::File { path });
        assert!(id.entries.is_empty());
        assert_eq!(
            ResourceId::file("library.jar"),
            ResourceId::file(PathBuf::from("library.jar"))
        );
    }

    fn entry(name: &str) -> ResourceEntry {
        ResourceEntry(name.into())
    }

    #[test]
    fn identical_entries_in_different_roots_are_distinct() {
        let a = file("/project/a.jar").entry(entry("org/example/Foo.java"));
        let b = file("/project/b.jar").entry(entry("org/example/Foo.java"));
        assert_ne!(a, b);
        assert_eq!(HashSet::from([a.clone(), a, b]).len(), 2);
    }

    #[test]
    fn nesting_preserves_parent_identity_and_entry_boundaries() {
        let root = file("/project/app.zip");
        let jar = root.entry(entry("lib/a.jar"));
        let class = jar.entry(entry("org/example/Foo.class"));
        assert!(root.entries.is_empty());
        assert_eq!(jar.entries.len(), 1);
        assert_eq!(class.entries.len(), 2);
        assert_eq!(class.root, root.root);
        assert_ne!(class, root.entry(entry("lib/a.jar/org/example/Foo.class")));
        assert_ne!(
            class,
            root.entry(entry("org/example/Foo.class"))
                .entry(entry("lib/a.jar"))
        );
    }

    #[test]
    fn virtual_keys_are_scoped_by_provider() {
        let virtual_resource = |provider: &str| {
            ResourceId::new(ResourceRoot::Virtual {
                provider: provider.into(),
                key: "Example.java".into(),
            })
        };
        assert_eq!(virtual_resource("generated"), virtual_resource("generated"));
        assert_ne!(virtual_resource("generated"), virtual_resource("scratch"));
    }
}
