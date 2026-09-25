use crate::{
    engine::JavaEngine,
    model::nodes::NodeKind,
    semantics::resolution::{
        Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate, query::ResolutionQuery,
        resolve, resolve_supertype,
    },
};
use beans_core::{
    engine::Revision,
    model::{
        classpath::{Classpath, ClasspathElement},
        names::Name,
        source::Source,
    },
    resolution::query::TypeDefinitionQuery,
};
use beans_platform_jvm::{
    engine::{JvmEngine, query::ClassHandle},
    model::{
        classes::{AccessLevel, Class, ClassKind},
        names::BinaryName,
    },
};

struct JvmDefinitions(Vec<(Name, ClassHandle)>);

impl TypeDefinitionQuery<ClassHandle> for JvmDefinitions {
    fn find_types<'a>(&'a self, name: &'a Name) -> impl Iterator<Item = ClassHandle> + 'a {
        self.0
            .iter()
            .filter(move |(candidate, _)| candidate == name)
            .map(|(_, handle)| handle.clone())
    }
}

#[derive(Clone, Copy)]
enum Occurrence {
    Field,
    Supertype,
}

struct Fixture {
    java: JavaEngine,
    source: Source,
    classpath: Classpath,
}

impl Fixture {
    fn new(current: &str, other_files: &[(&str, &str)]) -> Self {
        let mut java = JavaEngine::default();
        let source = Source::uri("file:///src/Use.java");
        java.store(Revision::new(1), java.process(current), source.clone());
        for (name, text) in other_files {
            java.store(
                Revision::new(1),
                java.process(text),
                Source::uri(format!("file:///src/{name}.java")),
            );
        }
        Self {
            java,
            source,
            classpath: Classpath::new(vec![ClasspathElement::new("/src".into(), [0; 32].into())]),
        }
    }

    fn resolve(&self, jvm: &JvmDefinitions) -> Result<TypeCandidate, ResolutionFailure> {
        self.resolve_at(jvm, Occurrence::Field)
    }

    fn resolve_at(
        &self,
        jvm: &JvmDefinitions,
        occurrence: Occurrence,
    ) -> Result<TypeCandidate, ResolutionFailure> {
        let java = self.java.query(Revision::new(1), Some(&self.classpath));
        let definitions = ResolutionQuery::new(&java, jvm);
        let file = java.file(&self.source).unwrap();
        let (index, reference) = file
            .iter_nodes()
            .find_map(|entry| match (occurrence, entry.node.kind()) {
                (Occurrence::Field, NodeKind::Field(field)) => {
                    Some((entry.index, field.declared_type.value()))
                }
                (Occurrence::Supertype, NodeKind::Type(declaration))
                    if declaration.name() == Some("Use") =>
                {
                    declaration
                        .declared_superclass
                        .as_ref()
                        .map(|reference| (entry.index, reference.value()))
                }
                _ => None,
            })
            .unwrap();
        let ctx = Context::new(Revision::new(1), &self.source, file, index, reference)
            .with_definitions(&definitions);
        match occurrence {
            Occurrence::Field => resolve(&ctx),
            Occurrence::Supertype => resolve_supertype(&ctx),
        }
    }

    fn expected(&self, name: &str) -> TypeCandidate {
        let query = self.java.query(Revision::new(1), Some(&self.classpath));
        let entry = query
            .find_type(&name.split('.').map(str::to_owned).collect())
            .into_iter()
            .next()
            .expect("expected a stored Java type");
        TypeCandidate::Java(JavaTypeCandidate::Declaration(
            query.declaration_handle(entry.source, entry.node_index),
        ))
    }
}

fn mock_jvm_class(name: &str) -> (JvmEngine, JvmDefinitions) {
    let mut jvm = JvmEngine::default();
    let binary_name = BinaryName::new(name);
    jvm.store(
        Revision::new(1),
        vec![Class::new(
            binary_name.clone(),
            ClassKind::Class,
            AccessLevel::Public,
        )],
        Source::class_file("mock.class"),
    );
    let handle = jvm
        .query(Revision::new(1))
        .find_class(&binary_name)
        .next()
        .unwrap()
        .handle;
    let canonical = name.split('.').map(str::to_owned).collect();
    (jvm, JvmDefinitions(vec![(canonical, handle)]))
}

#[test]
fn single_import_shadows_current_package_and_on_demand_imports() {
    let fixture = Fixture::new(
        "package p; import q.Foo; import r.*; class Use { Foo field; }",
        &[
            ("Same", "package p; class Foo {}"),
            ("Direct", "package q; public class Foo {}"),
            ("Demand", "package r; public class Foo {}"),
        ],
    );
    assert_eq!(
        fixture.resolve(&JvmDefinitions(vec![])),
        Ok(fixture.expected("q.Foo"))
    );
}

#[test]
fn current_package_shadows_on_demand_imports() {
    let fixture = Fixture::new(
        "package p; import q.*; class Use { Foo field; }",
        &[
            ("Same", "package p; class Foo {}"),
            ("Demand", "package q; public class Foo {}"),
        ],
    );
    assert_eq!(
        fixture.resolve(&JvmDefinitions(vec![])),
        Ok(fixture.expected("p.Foo"))
    );
}

#[test]
fn distinct_on_demand_imports_are_ambiguous_but_duplicate_imports_are_not() {
    let files = &[
        ("First", "package q; public class Foo {}"),
        ("Second", "package r; public class Foo {}"),
    ];
    let ambiguous = Fixture::new(
        "package p; import q.*; import r.*; class Use { Foo field; }",
        files,
    );
    let Err(ResolutionFailure::Ambiguous(candidates)) = ambiguous.resolve(&JvmDefinitions(vec![]))
    else {
        panic!("expected ambiguous on-demand imports");
    };
    assert_eq!(
        candidates,
        [ambiguous.expected("q.Foo"), ambiguous.expected("r.Foo")]
    );

    let duplicate = Fixture::new(
        "package p; import q.*; import q.*; class Use { Foo field; }",
        files,
    );
    assert_eq!(
        duplicate.resolve(&JvmDefinitions(vec![])),
        Ok(duplicate.expected("q.Foo"))
    );
}

#[test]
fn a_qualified_suffix_does_not_disambiguate_on_demand_imports() {
    let fixture = Fixture::new(
        "package p; import q.*; import r.*; class Use { Outer.Inner field; }",
        &[
            (
                "First",
                "package q; public class Outer { public class Inner {} }",
            ),
            ("Second", "package r; public class Outer {}"),
        ],
    );
    let Err(ResolutionFailure::Ambiguous(candidates)) = fixture.resolve(&JvmDefinitions(vec![]))
    else {
        panic!("expected two competing Outer types");
    };
    assert_eq!(
        candidates,
        [fixture.expected("q.Outer"), fixture.expected("r.Outer")]
    );
}

#[test]
fn java_lang_is_imported_on_demand() {
    let fixture = Fixture::new(
        "package p; class Use { String field; }",
        &[("String", "package java.lang; public class String {}")],
    );
    assert_eq!(
        fixture.resolve(&JvmDefinitions(vec![])),
        Ok(fixture.expected("java.lang.String"))
    );
}

#[test]
fn imported_qualified_name_resolves_the_member_and_commits_to_its_owner() {
    let fixture = Fixture::new(
        "package p; import q.Outer; class Use { Outer.Inner field; }",
        &[(
            "Outer",
            "package q; public class Outer { public class Inner {} }",
        )],
    );
    assert_eq!(
        fixture.resolve(&JvmDefinitions(vec![])),
        Ok(fixture.expected("q.Outer.Inner"))
    );

    let missing = Fixture::new(
        "package p; import q.Outer; class Use { Outer.Missing field; }",
        &[("Outer", "package q; public class Outer {}")],
    );
    assert_eq!(
        missing.resolve(&JvmDefinitions(vec![])),
        Err(ResolutionFailure::Partial)
    );
}

#[test]
fn qualified_spelling_can_start_with_a_package() {
    let fixture = Fixture::new(
        "package p; class Use { q.Outer.Inner field; }",
        &[(
            "Outer",
            "package q; public class Outer { public class Inner {} }",
        )],
    );
    assert_eq!(
        fixture.resolve(&JvmDefinitions(vec![])),
        Ok(fixture.expected("q.Outer.Inner"))
    );
}

#[test]
fn imported_superclass_uses_the_same_external_query() {
    let fixture = Fixture::new(
        "package p; import q.Base; class Use extends Base {}",
        &[("Base", "package q; public class Base {}")],
    );
    assert_eq!(
        fixture.resolve_at(&JvmDefinitions(vec![]), Occurrence::Supertype),
        Ok(fixture.expected("q.Base"))
    );
}

#[test]
fn the_query_can_supply_a_jvm_candidate_without_a_java_definition() {
    let fixture = Fixture::new("package p; import q.Foo; class Use { Foo field; }", &[]);
    let (_jvm, definitions) = mock_jvm_class("q.Foo");
    assert_eq!(
        fixture.resolve(&definitions),
        Ok(TypeCandidate::Jvm(definitions.0[0].1.clone()))
    );
}

#[test]
fn a_jvm_single_import_shadows_a_java_type_in_the_current_package() {
    let fixture = Fixture::new(
        "package p; import q.Foo; class Use { Foo field; }",
        &[("Foo", "package p; class Foo {}")],
    );
    let (_jvm, definitions) = mock_jvm_class("q.Foo");
    assert_eq!(
        fixture.resolve(&definitions),
        Ok(TypeCandidate::Jvm(definitions.0[0].1.clone()))
    );
}

#[test]
fn java_and_jvm_definitions_at_the_same_tier_are_not_silently_preferred() {
    let fixture = Fixture::new(
        "package p; import q.Foo; class Use { Foo field; }",
        &[("Foo", "package q; public class Foo {}")],
    );
    let (_jvm, definitions) = mock_jvm_class("q.Foo");
    let Err(ResolutionFailure::Ambiguous(candidates)) = fixture.resolve(&definitions) else {
        panic!("expected two distinct definitions");
    };
    assert_eq!(
        candidates,
        [
            fixture.expected("q.Foo"),
            TypeCandidate::Jvm(definitions.0[0].1.clone())
        ]
    );
}

#[test]
fn an_unmatched_name_is_not_found() {
    let fixture = Fixture::new("package p; class Use { Missing field; }", &[]);
    assert_eq!(
        fixture.resolve(&JvmDefinitions(vec![])),
        Err(ResolutionFailure::NotFound)
    );
}
