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
