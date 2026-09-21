use beans_core_model::names::Name;
use beans_core_resolution::query::TypeDefinitionQuery;
use beans_lang_java_semantics::query::DeclarationHandle;
use beans_platform_jvm_semantics::query::ClassHandle;

use crate::{JavaTypeCandidate, TypeCandidate};

pub struct ResolutionQuery<'a, Java, Jvm> {
    java: &'a Java,
    jvm: &'a Jvm,
}

impl<'a, Java, Jvm> ResolutionQuery<'a, Java, Jvm> {
    pub fn new(java: &'a Java, jvm: &'a Jvm) -> Self {
        Self { java, jvm }
    }
}

impl<Java, Jvm> TypeDefinitionQuery<TypeCandidate> for ResolutionQuery<'_, Java, Jvm>
where
    Java: TypeDefinitionQuery<DeclarationHandle>,
    Jvm: TypeDefinitionQuery<ClassHandle>,
{
    fn find_types<'query>(
        &'query self,
        name: &'query Name,
    ) -> impl Iterator<Item = TypeCandidate> + 'query {
        let java = self
            .java
            .find_types(name)
            .map(|handle| TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)));
        let jvm = self.jvm.find_types(name).map(TypeCandidate::Jvm);

        java.chain(jvm)
    }
}

#[cfg(test)]
mod tests {
    use beans_core_engine::{Revision, storage::RevisionedStorage};
    use beans_core_model::source::Source;
    use beans_lang_java_model::File;
    use beans_platform_jvm_model::{
        classes::{AccessLevel, Class, ClassKind},
        names::BinaryName,
    };
    use beans_platform_jvm_semantics::query::JvmQuery;

    use super::*;

    struct JavaDefinitions(Vec<DeclarationHandle>);

    impl TypeDefinitionQuery<DeclarationHandle> for JavaDefinitions {
        fn find_types<'query>(
            &'query self,
            _name: &'query Name,
        ) -> impl Iterator<Item = DeclarationHandle> + 'query {
            self.0.clone().into_iter()
        }
    }

    struct JvmDefinitions(Vec<ClassHandle>);

    impl TypeDefinitionQuery<ClassHandle> for JvmDefinitions {
        fn find_types<'query>(
            &'query self,
            _name: &'query Name,
        ) -> impl Iterator<Item = ClassHandle> + 'query {
            self.0.clone().into_iter()
        }
    }

    #[test]
    fn native_definition_handles_are_wrapped_and_chained_in_query_order() {
        let revision = Revision::new(1);
        let java_handle = DeclarationHandle::new(
            revision,
            Source::SourceFile {
                path: "src/Example.java".into(),
            },
            File::ROOT_NODE_ID,
        );

        let binary_name = BinaryName::new("example.Example");
        let mut classes = RevisionedStorage::default();
        classes.put(
            revision,
            Source::ClassFile {
                path: "lib/Example.class".into(),
            },
            vec![Class::new(
                binary_name.clone(),
                ClassKind::Class,
                AccessLevel::Public,
            )],
        );
        let jvm_query = JvmQuery::new(&classes, revision);
        let jvm_handle = jvm_query
            .find_class(&binary_name)
            .next()
            .expect("expected the stored class")
            .handle;

        let java = JavaDefinitions(vec![java_handle.clone()]);
        let jvm = JvmDefinitions(vec![jvm_handle.clone()]);
        let query = ResolutionQuery::new(&java, &jvm);
        let name = Name::new(vec!["example".into(), "Example".into()]);

        let candidates: Vec<_> = query.find_types(&name).collect();

        assert_eq!(
            candidates,
            [
                TypeCandidate::Java(JavaTypeCandidate::Declaration(java_handle)),
                TypeCandidate::Jvm(jvm_handle),
            ]
        );
    }
}
