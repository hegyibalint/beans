use crate::query::JavaQuery;
use beans_core::{names::Name, query_scope::QueryScope, resolution::TypeDefinitionQuery};
use beans_platform_jvm::query::JvmQuery;
use rusqlite::Connection;

use super::{JavaTypeCandidate, TypeCandidate};

pub struct ResolutionQuery<'a> {
    java: JavaQuery<'a>,
    jvm: JvmQuery<'a>,
}

impl<'a> ResolutionQuery<'a> {
    pub fn new(connection: &'a Connection, scope: QueryScope<'a>) -> Self {
        Self {
            java: JavaQuery::new(connection, scope),
            jvm: JvmQuery::new(connection, scope),
        }
    }
}

impl TypeDefinitionQuery<TypeCandidate> for ResolutionQuery<'_> {
    fn find_types(&self, name: &Name) -> Result<Vec<TypeCandidate>, String> {
        let java = self
            .java
            .find_types(name)?
            .into_iter()
            .map(|handle| TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)));
        // JVMS §4.2.1: internal names use '/'; canonical member-name conversion is not wired.
        let binary_name = name.as_slice().join("/");
        let jvm = self
            .jvm
            .find_classes(&binary_name)?
            .into_iter()
            .map(TypeCandidate::Jvm);
        Ok(java.chain(jvm).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Models;
    use beans_core::{classpath::Unrestricted, location::Location};

    #[test]
    fn native_java_addresses_are_wrapped_as_candidates() {
        let name = Name::new(vec!["example".into(), "Example".into()]);
        let mut models = Models::new();
        models.store(
            models.process("package example; class Example {}"),
            Location::file("/src/Example.java"),
        );
        let java = models.query(QueryScope::new(&Unrestricted));
        let address = java.find_types(&name).unwrap().remove(0);
        let query = ResolutionQuery::new(&models.connection, QueryScope::new(&Unrestricted));
        assert_eq!(
            query.find_types(&name).unwrap(),
            [TypeCandidate::Java(JavaTypeCandidate::Declaration(address))]
        );
    }

    #[test]
    fn jvm_storage_errors_are_not_missing_candidates() {
        let models = Models::new();
        models.connection.execute("DROP TABLE models", []).unwrap();
        let query = ResolutionQuery::new(&models.connection, QueryScope::new(&Unrestricted));
        assert!(
            query
                .find_types(&Name::new(vec!["Missing".into()]))
                .is_err()
        );
    }

    #[test]
    fn java_storage_errors_are_not_missing_candidates() {
        let models = Models::new();
        models
            .connection
            .execute("DROP TABLE java_types", [])
            .unwrap();
        let query = ResolutionQuery::new(&models.connection, QueryScope::new(&Unrestricted));
        assert!(
            query
                .find_types(&Name::new(vec!["Missing".into()]))
                .is_err()
        );
    }
}
