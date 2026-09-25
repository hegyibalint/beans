use crate::semantics::DeclarationHandle;
use beans_core::model::names::Name;
use beans_core::resolution::query::TypeDefinitionQuery;
use beans_platform_jvm::engine::query::ClassHandle;

use super::{JavaTypeCandidate, TypeCandidate};

/// Object-safe view of the combined query for a resolver context. `TypeDefinitionQuery`
/// returns an opaque iterator, so it cannot be borrowed as a trait object.
pub trait TypeCandidateQuery {
    fn candidates(&self, name: &Name) -> Vec<TypeCandidate>;
}

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

impl<Java, Jvm> TypeCandidateQuery for ResolutionQuery<'_, Java, Jvm>
where
    Java: TypeDefinitionQuery<DeclarationHandle>,
    Jvm: TypeDefinitionQuery<ClassHandle>,
{
    fn candidates(&self, name: &Name) -> Vec<TypeCandidate> {
        TypeDefinitionQuery::find_types(self, name).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::model::File;
    use beans_core::engine::{Revision, storage::RevisionedStorage};
    use beans_core::model::source::Source;
    use beans_platform_jvm::engine::query::JvmQuery;
    use beans_platform_jvm::model::{
        classes::{AccessLevel, Class, ClassKind},
        names::BinaryName,
    };

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
            Source::uri("file:///src/Example.java"),
            File::ROOT_NODE_ID,
        );

        let binary_name = BinaryName::new("example.Example");
        let mut classes = RevisionedStorage::default();
        classes.put(
            revision,
            Source::class_file("lib/Example.class"),
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
