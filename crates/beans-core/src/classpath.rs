use crate::resource::ResourceId;

/// Determines which resources are available to a query.
/// Membership does not resolve symbols or check language-level accessibility.
pub trait Classpath {
    fn contains_source(&self, resource: &ResourceId) -> bool;
    fn contains_class(&self, resource: &ResourceId) -> bool;
}

/// Unscoped queries see every resource.
pub struct Unrestricted;

impl Classpath for Unrestricted {
    fn contains_source(&self, _resource: &ResourceId) -> bool {
        true
    }

    fn contains_class(&self, _resource: &ResourceId) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{ResourceEntry, ResourceRoot};

    #[test]
    fn unrestricted_includes_files_virtual_resources_and_nested_entries() {
        let classpath = Unrestricted;
        let file = ResourceId::new(ResourceRoot::File {
            path: "/src/Example.java".into(),
        });
        let virtual_resource = ResourceId::new(ResourceRoot::Virtual {
            provider: "scratch".into(),
            key: "Example.java".into(),
        });
        let class = ResourceId::new(ResourceRoot::File {
            path: "/library.jar".into(),
        })
        .entry(ResourceEntry("p/Example.class".into()));

        for resource in [file, virtual_resource, class] {
            assert!(classpath.contains_source(&resource));
            assert!(classpath.contains_class(&resource));
        }
    }
}
