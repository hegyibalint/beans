use crate::origin::Origin;

/// Determines which origins are available to a query.
/// Membership does not resolve symbols or check language-level accessibility.
pub trait Classpath {
    fn contains(&self, origin: &Origin) -> bool;
}

/// Unscoped queries see every origin.
pub struct Unrestricted;

impl Classpath for Unrestricted {
    fn contains(&self, _origin: &Origin) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrestricted_includes_source_files_virtual_documents_and_binary_origins() {
        let classpath = Unrestricted;

        assert!(classpath.contains(&Origin::uri("file:///src/Example.java")));
        assert!(classpath.contains(&Origin::uri("untitled:Example.java")));
        assert!(classpath.contains(&Origin::jar_entry("library.jar", "p/Example.class")));
    }
}
