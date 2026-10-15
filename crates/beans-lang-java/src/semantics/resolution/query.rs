use crate::query::JavaQuery;
use beans_core::{names::Name, query_scope::QueryScope, resolution::TypeDefinitionQuery};
use beans_platform_jvm::JvmPlatform;

use super::{JavaTypeCandidate, TypeCandidate};

pub struct ResolutionQuery<'a> {
    java: &'a JavaQuery<'a>,
    jvm: &'a JvmPlatform,
}

impl<'a> ResolutionQuery<'a> {
    pub fn new(java: &'a JavaQuery<'a>, jvm: &'a JvmPlatform) -> Self {
        Self { java, jvm }
    }
}

impl TypeDefinitionQuery<TypeCandidate> for ResolutionQuery<'_> {
    fn find_types(&self, name: &Name) -> Result<Vec<TypeCandidate>, String> {
        let java = self
            .java
            .find_types(name)?
            .into_iter()
            .map(|handle| TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)));
        let scope = QueryScope::new(self.java.revision(), self.java.classpath());
        // JVMS §4.2.1: internal names use '/'; canonical member-name conversion is not wired.
        let binary_name = name.as_slice().join("/");
        let jvm = self
            .jvm
            .find_classes(scope, &binary_name)
            .into_iter()
            .map(TypeCandidate::Jvm);
        Ok(java.chain(jvm).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Models;
    use beans_core::{classpath::Unrestricted, location::Location, revision::Revision};

    #[test]
    fn native_java_addresses_are_wrapped_as_candidates() {
        let revision = Revision::new(1);
        let name = Name::new(vec!["example".into(), "Example".into()]);
        let mut models = Models::new();
        models.store(
            revision,
            models.process("package example; class Example {}"),
            Location::file("/src/Example.java"),
        );
        let java = models.query(QueryScope::new(revision, &Unrestricted));
        let address = java.find_types(&name).unwrap().remove(0);
        let jvm = JvmPlatform::new();
        let query = ResolutionQuery::new(&java, &jvm);
        assert_eq!(
            query.find_types(&name).unwrap(),
            [TypeCandidate::Java(JavaTypeCandidate::Declaration(address))]
        );
    }

    #[test]
    fn storage_errors_are_not_missing_candidates() {
        let models = Models::new();
        models
            .connection
            .execute("DROP TABLE java_types", [])
            .unwrap();
        let java = models.query(QueryScope::new(Revision::new(1), &Unrestricted));
        let jvm = JvmPlatform::new();
        let query = ResolutionQuery::new(&java, &jvm);
        assert!(
            query
                .find_types(&Name::new(vec!["Missing".into()]))
                .is_err()
        );
    }
}
