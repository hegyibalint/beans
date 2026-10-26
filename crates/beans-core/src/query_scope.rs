use crate::{classpath::Classpath, location::Location};

/// Selects which locations a query can see; the database transaction owns its snapshot.
#[derive(Clone, Copy)]
pub struct QueryScope<'a> {
    sources: &'a [Classpath],
    classes: &'a [Classpath],
}

impl<'a> QueryScope<'a> {
    pub fn new(sources: &'a [Classpath], classes: &'a [Classpath]) -> Self {
        Self { sources, classes }
    }

    pub fn unrestricted() -> Self {
        const ENTRIES: &[Classpath] = &[Classpath::Unrestricted];
        Self::new(ENTRIES, ENTRIES)
    }

    pub fn contains_source(self, resource: &Location) -> bool {
        self.sources
            .iter()
            .any(|entry| entry.contains_source(resource))
    }

    pub fn contains_class(self, resource: &Location) -> bool {
        self.classes
            .iter()
            .any(|entry| entry.contains_class(resource))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::LocationEntry;

    #[test]
    fn query_scope_borrows_and_combines_separate_classpath_lists() {
        let sources = [Classpath::file("/src"), Classpath::file("/shared")];
        let classes = [
            Classpath::file("/first.jar"),
            Classpath::file("/second.jar"),
        ];
        let scope = QueryScope::new(&sources, &classes);
        assert!(std::ptr::eq(scope.sources, sources.as_slice()));
        assert!(std::ptr::eq(scope.classes, classes.as_slice()));

        for path in ["/src/C.java", "/shared/C.java"] {
            let source = Location::file(path);
            assert!(scope.contains_source(&source));
            assert!(!scope.contains_class(&source));
        }
        for path in ["/first.jar", "/second.jar"] {
            let class = Location::file(path).entry(LocationEntry::Container("p/C.class".into()));
            assert!(!scope.contains_source(&class));
            assert!(scope.contains_class(&class));
        }
        assert!(!scope.contains_source(&Location::file("/other/C.java")));
        assert!(!scope.contains_class(&Location::file("/other/C.class")));
    }

    #[test]
    fn empty_lists_accept_nothing_while_unrestricted_accepts_everything() {
        for resource in [
            Location::file("/C.java"),
            Location::new(LocationEntry::Virtual("untitled:C.java".into())),
            Location::file("/library.jar").entry(LocationEntry::Container("p/C.class".into())),
        ] {
            assert!(!QueryScope::new(&[], &[]).contains_source(&resource));
            assert!(!QueryScope::new(&[], &[]).contains_class(&resource));
            assert!(QueryScope::unrestricted().contains_source(&resource));
            assert!(QueryScope::unrestricted().contains_class(&resource));
        }
    }
}
