use std::collections::HashMap;

use beans_core::{
    classpath::{Classpath, Unrestricted},
    query_scope::QueryScope,
    resource::ResourceId,
    revision::Revision,
};
use beans_lang_java::{
    lowering::lower_into,
    model::{File, nodes::NodeKind},
    model_query::JavaModelQuery,
    query::JavaQuery,
    semantics::resolution::{
        Context, JavaTypeCandidate, TypeCandidate, query::ResolutionQuery, resolve,
    },
};
use beans_platform_jvm::{Class, model_query::JvmModelQuery};

struct Models {
    files: HashMap<ResourceId, File>,
}

impl JavaModelQuery for Models {
    fn java_file(&self, source: &ResourceId) -> Option<&File> {
        self.files.get(source)
    }

    fn java_files(&self) -> impl Iterator<Item = (&ResourceId, &File)> {
        self.files.iter()
    }
}

impl JvmModelQuery for Models {
    fn jvm_class(&self, _source: &ResourceId) -> Option<&Class> {
        None
    }

    fn jvm_classes(&self) -> impl Iterator<Item = (&ResourceId, &Class)> {
        std::iter::empty()
    }
}

#[test]
fn supplied_models_can_be_resolved_using_the_java_query() {
    let source = ResourceId::file("/src/app/Use.java");
    let models = Models {
        files: HashMap::from([(
            source.clone(),
            lower_into("package app; class Target {} class Use { Target target; }"),
        )]),
    };
    let revision = Revision::new(3);
    let classpath = Unrestricted;
    let query = JavaQuery::new(&models, QueryScope::new(revision, &classpath));

    assert_eq!(query.revision(), revision);
    assert!(std::ptr::eq(
        query.classpath(),
        &classpath as &dyn Classpath
    ));
    let file = query.file(&source).expect("expected the supplied model");
    let (node_index, type_ref) = file
        .iter_nodes()
        .find_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) if field.name == "target" => {
                Some((entry.index, &field.declared_type))
            }
            _ => None,
        })
        .expect("expected target field");
    let ctx = Context::new(revision, &source, file, node_index, type_ref.value());
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(declaration)) = resolve(&ctx).unwrap()
    else {
        panic!("expected a Java declaration");
    };
    let target = query
        .declaration(&declaration)
        .expect("expected a readable declaration handle");

    assert_eq!(target.declaration.name(), Some("Target"));
    assert_eq!(target.source, &source);
}

#[test]
fn an_imported_type_resolves_to_another_supplied_source_file() {
    let use_source = ResourceId::file("/src/p/Use.java");
    let target_source = ResourceId::file("/src/q/Target.java");
    let models = Models {
        files: HashMap::from([
            (
                use_source.clone(),
                lower_into("package p; import q.Target; class Use { Target field; }"),
            ),
            (
                target_source.clone(),
                lower_into("package q; public class Target {}"),
            ),
        ]),
    };
    let revision = Revision::new(1);
    let classpath = Unrestricted;
    let java = JavaQuery::new(&models, QueryScope::new(revision, &classpath));
    let definitions = ResolutionQuery::new(&java, &models);
    let file = java.file(&use_source).unwrap();
    let field = file
        .iter_nodes()
        .find_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) => Some((entry.index, field)),
            _ => None,
        })
        .unwrap();
    let ctx = Context::new(
        revision,
        &use_source,
        file,
        field.0,
        field.1.declared_type.value(),
    )
    .with_definitions(&definitions);

    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = resolve(&ctx).unwrap() else {
        panic!("expected a supplied Java type");
    };
    assert_eq!(handle.source(), &target_source);
    assert_eq!(
        java.declaration(&handle).unwrap().declaration.name(),
        Some("Target")
    );
}
