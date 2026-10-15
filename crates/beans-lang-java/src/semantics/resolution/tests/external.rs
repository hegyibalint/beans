use std::io::Write;

use crate::{
    model::nodes::NodeKind,
    semantics::resolution::{
        Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate, query::ResolutionQuery,
        resolve, resolve_supertype,
    },
    testing::Models,
};
use beans_core::{
    classpath::Unrestricted, location::Location, query_scope::QueryScope, revision::Revision,
};
use beans_platform_jvm::JvmPlatform;
use beans_testing::support::class_bytes;
use tempfile::NamedTempFile;

#[derive(Clone, Copy)]
enum Occurrence {
    Field,
    Supertype,
}

struct Fixture {
    java: Models,
    source: Location,
}

impl Fixture {
    fn new(current: &str, other_files: &[(&str, &str)]) -> Self {
        let mut java = Models::new();
        let source = Location::file("/src/Use.java");
        java.store(Revision::new(1), java.process(current), source.clone());
        for (name, text) in other_files {
            java.store(
                Revision::new(1),
                java.process(text),
                Location::file(format!("/src/{name}.java")),
            );
        }
        Self { java, source }
    }

    fn resolve(&self, jvm: &JvmPlatform) -> Result<TypeCandidate, ResolutionFailure> {
        self.resolve_at(jvm, Occurrence::Field)
    }

    fn resolve_at(
        &self,
        jvm: &JvmPlatform,
        occurrence: Occurrence,
    ) -> Result<TypeCandidate, ResolutionFailure> {
        let java = self
            .java
            .query(QueryScope::new(Revision::new(1), &Unrestricted));
        let definitions = ResolutionQuery::new(&java, jvm);
        let file = java.file(&self.source).unwrap().unwrap();
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
        let ctx = Context::new(Revision::new(1), &self.source, &file, index, reference)
            .with_definitions(&definitions);
        match occurrence {
            Occurrence::Field => resolve(&ctx),
            Occurrence::Supertype => resolve_supertype(&ctx),
        }
    }

    fn expected(&self, name: &str) -> TypeCandidate {
        let query = self
            .java
            .query(QueryScope::new(Revision::new(1), &Unrestricted));
        let entry = query
            .find_type(&name.split('.').map(str::to_owned).collect())
            .unwrap()
            .into_iter()
            .next()
            .expect("expected a stored Java type");
        TypeCandidate::Java(JavaTypeCandidate::Declaration(entry))
    }
}

fn jvm_class(binary_name: &str) -> (NamedTempFile, JvmPlatform) {
    let revision = Revision::new(1);
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&class_bytes(binary_name)).unwrap();
    let mut jvm = JvmPlatform::new();
    jvm.process(revision, file.path().to_owned()).unwrap();
    (file, jvm)
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
        fixture.resolve(&JvmPlatform::new()),
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
        fixture.resolve(&JvmPlatform::new()),
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
    let Err(ResolutionFailure::Ambiguous(candidates)) = ambiguous.resolve(&JvmPlatform::new())
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
        duplicate.resolve(&JvmPlatform::new()),
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
    let Err(ResolutionFailure::Ambiguous(candidates)) = fixture.resolve(&JvmPlatform::new()) else {
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
        fixture.resolve(&JvmPlatform::new()),
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
        fixture.resolve(&JvmPlatform::new()),
        Ok(fixture.expected("q.Outer.Inner"))
    );

    let missing = Fixture::new(
        "package p; import q.Outer; class Use { Outer.Missing field; }",
        &[("Outer", "package q; public class Outer {}")],
    );
    assert_eq!(
        missing.resolve(&JvmPlatform::new()),
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
        fixture.resolve(&JvmPlatform::new()),
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
        fixture.resolve_at(&JvmPlatform::new(), Occurrence::Supertype),
        Ok(fixture.expected("q.Base"))
    );
}

#[test]
fn jvm_candidates_are_unavailable_until_persistence_is_implemented() {
    let fixture = Fixture::new("package p; import q.Foo; class Use { Foo field; }", &[]);
    let (_file, jvm) = jvm_class("q/Foo");
    assert_eq!(fixture.resolve(&jvm), Err(ResolutionFailure::NotFound));
}

#[test]
fn deferred_jvm_candidates_do_not_shadow_java_types() {
    let fixture = Fixture::new(
        "package p; import q.Foo; class Use { Foo field; }",
        &[("Foo", "package p; class Foo {}")],
    );
    let (_file, jvm) = jvm_class("q/Foo");
    assert_eq!(fixture.resolve(&jvm), Ok(fixture.expected("p.Foo")));
}

#[test]
fn deferred_jvm_candidates_do_not_make_java_types_ambiguous() {
    let fixture = Fixture::new(
        "package p; import q.Foo; class Use { Foo field; }",
        &[("Foo", "package q; public class Foo {}")],
    );
    let (_file, jvm) = jvm_class("q/Foo");
    assert_eq!(fixture.resolve(&jvm), Ok(fixture.expected("q.Foo")));
}

#[test]
fn an_unmatched_name_is_not_found() {
    let fixture = Fixture::new("package p; class Use { Missing field; }", &[]);
    assert_eq!(
        fixture.resolve(&JvmPlatform::new()),
        Err(ResolutionFailure::NotFound)
    );
}
