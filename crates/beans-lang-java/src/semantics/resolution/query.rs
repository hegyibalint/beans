use crate::{model_query::JavaModelQuery, query::JavaQuery};
use beans_core::{names::Name, query_scope::QueryScope, resolution::TypeDefinitionQuery};
use beans_platform_jvm::JvmPlatform;

use super::{JavaTypeCandidate, TypeCandidate};

pub struct ResolutionQuery<'a, Q: JavaModelQuery> {
    java: &'a JavaQuery<'a, Q>,
    jvm: &'a JvmPlatform,
}

impl<'a, Q: JavaModelQuery> ResolutionQuery<'a, Q> {
    pub fn new(java: &'a JavaQuery<'a, Q>, jvm: &'a JvmPlatform) -> Self {
        Self { java, jvm }
    }
}

/// Object-safe view of the combined query for a resolver context. `TypeDefinitionQuery`
/// returns an opaque iterator, so it cannot be borrowed as a trait object.
pub trait TypeCandidateQuery {
    fn candidates(&self, name: &Name) -> Vec<TypeCandidate>;
}

impl<Q: JavaModelQuery> TypeDefinitionQuery<TypeCandidate> for ResolutionQuery<'_, Q> {
    fn find_types<'query>(
        &'query self,
        name: &'query Name,
    ) -> impl Iterator<Item = TypeCandidate> + 'query {
        let java = self
            .java
            .find_types(name)
            .map(|handle| TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)));
        let scope = QueryScope::new(self.java.revision(), self.java.classpath());
        // JVMS §4.2.1: internal names use '/'; canonical member-name conversion is not wired.
        let binary_name = name.as_slice().join("/");
        let jvm = self
            .jvm
            .find_classes(scope, &binary_name)
            .into_iter()
            .map(TypeCandidate::Jvm);

        java.chain(jvm)
    }
}

impl<Q: JavaModelQuery> TypeCandidateQuery for ResolutionQuery<'_, Q> {
    fn candidates(&self, name: &Name) -> Vec<TypeCandidate> {
        TypeDefinitionQuery::find_types(self, name).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use beans_core::{
        classpath::{Classpath, Unrestricted},
        resource::ResourceId,
        revision::Revision,
    };
    use beans_storage::RevisionEntry;
    use beans_testing::support::class_bytes;
    use tempfile::NamedTempFile;

    use crate::{JavaLanguage, lower_into, testing::Models};

    use super::*;

    #[test]
    fn native_definition_addresses_are_wrapped_and_chained_in_query_order() {
        let revision = Revision::new(1);
        let name = Name::new(vec!["example".into(), "Example".into()]);
        let mut language = Models::new();
        language.store(
            revision,
            lower_into("package example; class Example {}"),
            ResourceId::file("/src/Example.java"),
        );
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&class_bytes("example/Example")).unwrap();
        let mut platform = JvmPlatform::new();
        platform.process(revision, file.path().to_owned()).unwrap();
        let models = language.model_query(revision);
        let java = JavaLanguage::new().query(&models, QueryScope::new(revision, &Unrestricted));
        let java_address = java.find_types(&name).next().unwrap();
        let query = ResolutionQuery::new(&java, &platform);

        let candidates: Vec<_> = query.find_types(&name).collect();

        assert_eq!(
            candidates,
            [
                TypeCandidate::Java(JavaTypeCandidate::Declaration(java_address)),
                TypeCandidate::Jvm(RevisionEntry {
                    revision,
                    key: ResourceId::file(file.path()),
                }),
            ]
        );
    }

    #[test]
    fn jvm_candidates_use_the_java_queries_revision_and_classpath() {
        struct Visibility(bool);

        impl Classpath for Visibility {
            fn contains_source(&self, _resource: &ResourceId) -> bool {
                true
            }

            fn contains_class(&self, _resource: &ResourceId) -> bool {
                self.0
            }
        }

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&class_bytes("example/Example")).unwrap();
        let mut platform = JvmPlatform::new();
        platform
            .process(Revision::new(5), file.path().to_owned())
            .unwrap();
        let language = Models::new();
        let name = Name::new(vec!["example".into(), "Example".into()]);

        for (revision, visible, expected) in [(4, true, false), (7, false, false), (7, true, true)]
        {
            let classpath = Visibility(visible);
            let revision = Revision::new(revision);
            let models = language.model_query(revision);
            let java = JavaLanguage::new().query(&models, QueryScope::new(revision, &classpath));
            let query = ResolutionQuery::new(&java, &platform);
            let candidates = query.candidates(&name);

            if expected {
                assert_eq!(
                    candidates,
                    vec![TypeCandidate::Jvm(RevisionEntry {
                        revision,
                        key: ResourceId::file(file.path()),
                    })]
                );
            } else {
                assert!(candidates.is_empty());
            }
        }
    }
}
