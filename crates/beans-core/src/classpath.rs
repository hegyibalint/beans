use crate::location::Location;

/// Determines which resources are available to a query.
/// Membership does not resolve symbols or check language-level accessibility.
pub trait Classpath {
    fn contains_source(&self, resource: &Location) -> bool;
    fn contains_class(&self, resource: &Location) -> bool;
}

/// Unscoped queries see every resource.
pub struct Unrestricted;

impl Classpath for Unrestricted {
    fn contains_source(&self, _resource: &Location) -> bool {
        true
    }

    fn contains_class(&self, _resource: &Location) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::LocationEntry;

    #[test]
    fn unrestricted_includes_files_virtual_resources_and_nested_entries() {
        let classpath = Unrestricted;
        let file = Location::new(LocationEntry::File("/src/Example.java".into()));
        let virtual_resource =
            Location::new(LocationEntry::Virtual("untitled:Example.java".into()));
        let class = Location::new(LocationEntry::File("/library.jar".into()))
            .entry(LocationEntry::Container("p/Example.class".into()));

        for resource in [file, virtual_resource, class] {
            assert!(classpath.contains_source(&resource));
            assert!(classpath.contains_class(&resource));
        }
    }
}
