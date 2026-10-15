use crate::{model_query::JavaModelQuery, query::JavaQuery};
use beans_core::{names::Name, query_scope::QueryScope, resolution::TypeDefinitionQuery};
use beans_platform_jvm::model_query::JvmModelQuery;

use super::{JavaTypeCandidate, TypeCandidate};

pub struct ResolutionQuery<'a, J: JavaModelQuery, P: JvmModelQuery> {
    java: &'a JavaQuery<'a, J>,
    jvm: &'a P,
}

impl<'a, J: JavaModelQuery, P: JvmModelQuery> ResolutionQuery<'a, J, P> {
    pub fn new(java: &'a JavaQuery<'a, J>, jvm: &'a P) -> Self {
        Self { java, jvm }
    }
}

impl<J: JavaModelQuery, P: JvmModelQuery> TypeDefinitionQuery<TypeCandidate>
    for ResolutionQuery<'_, J, P>
{
    fn find_types(&self, name: &Name) -> Result<Vec<TypeCandidate>, String> {
        let java = self
            .java
            .find_types(name)?
            .into_iter()
            .map(|handle| TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)));
        let scope = QueryScope::new(self.java.revision(), self.java.classpath());
        // JVMS §4.2.1: internal names use '/'; canonical member-name conversion is not wired.
        let binary_name = name.as_slice().join("/");
        let jvm = beans_platform_jvm::find_classes(self.jvm, scope, &binary_name)?
            .into_iter()
            .map(TypeCandidate::Jvm);

        Ok(java.chain(jvm).collect())
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

    use crate::{lower_into, testing::Models};

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
        let source = ResourceId::file(file.path());
        for entry in beans_platform_jvm::process(&source, std::fs::File::open(file.path()).unwrap())
            .unwrap()
            .unwrap()
        {
            let (source, class) = entry.unwrap();
            language.store_class(revision, class, source);
        }
        let models = language.model_query(revision);
        let java = JavaQuery::new(&models, QueryScope::new(revision, &Unrestricted));
        let java_address = java.find_types(&name).unwrap().into_iter().next().unwrap();
        let query = ResolutionQuery::new(&java, &models);

        let candidates = query.find_types(&name).unwrap();

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
        let mut language = Models::new();
        let source = ResourceId::file(file.path());
        for entry in beans_platform_jvm::process(&source, std::fs::File::open(file.path()).unwrap())
            .unwrap()
            .unwrap()
        {
            let (source, class) = entry.unwrap();
            language.store_class(Revision::new(5), class, source);
        }
        let name = Name::new(vec!["example".into(), "Example".into()]);

        for (revision, visible, expected) in [(4, true, false), (7, false, false), (7, true, true)]
        {
            let classpath = Visibility(visible);
            let revision = Revision::new(revision);
            let models = language.model_query(revision);
            let java = JavaQuery::new(&models, QueryScope::new(revision, &classpath));
            let query = ResolutionQuery::new(&java, &models);
            let candidates = query.find_types(&name).unwrap();

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
