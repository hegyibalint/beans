use std::path::Path;

use beans_core::{
    compilation_scope::{CompilationScope, CompilationScopes},
    location::Location,
    query_scope::QueryScope,
    resolution::TypeDefinitionQuery,
    resource_filter::ResourceFilter,
};
use beans_lang_java::{
    model::nodes::NodeKind,
    semantics::resolution::{
        Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate, query::ResolutionQuery,
        resolve, resolve_supertype,
    },
};
use beans_platform_jvm::JvmPlatform;
use beans_testing::{TempDir, support::class_bytes};

use super::support::Models;

#[derive(Clone, Copy)]
enum Occurrence {
    Field,
    Supertype,
}

struct Fixture {
    java: Models,
    source: Location,
    scopes: CompilationScopes,
}

impl Fixture {
    fn new(current: &str, other_files: &[(&str, &str)]) -> Self {
        let mut fixture = Self::from_sources("/src/Use.java", &[("/src/Use.java", current)]);
        for (name, text) in other_files {
            fixture.java.store(
                fixture.java.process(text),
                Location::file(format!("/src/{name}.java")),
            );
        }
        fixture
    }

    fn from_sources(source: &str, sources: &[(&str, &str)]) -> Self {
        let mut java = Models::new();
        for (path, contents) in sources {
            java.store(java.process(contents), Location::file(*path));
        }
        Self {
            java,
            source: Location::file(source),
            scopes: CompilationScopes::unrestricted(),
        }
    }

    fn competing_targets() -> Self {
        let fixture = Self::from_sources(
            "/consumer/Use.java",
            &[
                ("/consumer/Use.java", "class Use { Target field; }"),
                ("/sourceset-a/Target.java", "class Target {}"),
                ("/sourceset-b/Target.java", "class Target {}"),
            ],
        );
        let locations: Vec<_> = fixture
            .prospective_candidates("Target")
            .into_iter()
            .map(|candidate| candidate.location().clone())
            .collect();
        assert_eq!(
            locations,
            [
                Location::file("/sourceset-a/Target.java"),
                Location::file("/sourceset-b/Target.java"),
            ]
        );
        fixture
    }

    fn resolve(&self) -> Result<TypeCandidate, ResolutionFailure> {
        self.resolve_at(Occurrence::Field)
    }

    fn resolve_at(&self, occurrence: Occurrence) -> Result<TypeCandidate, ResolutionFailure> {
        let java = self.java.query(QueryScope::unrestricted());
        let definitions = ResolutionQuery::new(&self.java.connection, QueryScope::unrestricted());
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
        let ctx = Context::new(&self.source, &file, index, reference)
            .with_definitions(&definitions)
            .with_scopes(&self.scopes);
        match occurrence {
            Occurrence::Field => resolve(&ctx),
            Occurrence::Supertype => resolve_supertype(&ctx),
        }
    }

    fn store_jvm_resource(&mut self, path: &Path) {
        let transaction = self.java.connection.transaction().unwrap();
        let processed = JvmPlatform::new().process(path).unwrap();
        assert!(processed.failures.is_empty());
        assert!(!processed.classes.is_empty());
        for (location, class) in processed.classes {
            beans_platform_jvm::storage::store(&transaction, &class, &location).unwrap();
        }
        transaction.commit().unwrap();
    }

    fn prospective_candidates(&self, name: &str) -> Vec<TypeCandidate> {
        let definitions = ResolutionQuery::new(&self.java.connection, QueryScope::unrestricted());
        definitions
            .find_types(&name.split('.').map(str::to_owned).collect())
            .unwrap()
    }

    fn expected(&self, name: &str) -> TypeCandidate {
        let query = self.java.query(QueryScope::unrestricted());
        let entry = query
            .find_type(&name.split('.').map(str::to_owned).collect())
            .unwrap()
            .into_iter()
            .next()
            .expect("expected a stored Java type");
        TypeCandidate::Java(JavaTypeCandidate::Declaration(entry))
    }
}

#[test]
fn dependency_on_sourceset_a_resolves_its_target() {
    let mut fixture = Fixture::competing_targets();
    fixture.scopes = CompilationScopes::new(vec![CompilationScope {
        inputs: vec![ResourceFilter::file("/consumer")],
        dependencies: Some(vec![ResourceFilter::file("/sourceset-a")]),
    }]);

    assert_eq!(
        fixture.resolve().unwrap().location(),
        &Location::file("/sourceset-a/Target.java")
    );
}

#[test]
fn dependency_on_sourceset_b_resolves_its_target() {
    let mut fixture = Fixture::competing_targets();
    fixture.scopes = CompilationScopes::new(vec![CompilationScope {
        inputs: vec![ResourceFilter::file("/consumer")],
        dependencies: Some(vec![ResourceFilter::file("/sourceset-b")]),
    }]);

    assert_eq!(
        fixture.resolve().unwrap().location(),
        &Location::file("/sourceset-b/Target.java")
    );
}

#[test]
fn dependencies_on_both_sourcesets_make_target_ambiguous() {
    let mut fixture = Fixture::competing_targets();
    fixture.scopes = CompilationScopes::new(vec![CompilationScope {
        inputs: vec![ResourceFilter::file("/consumer")],
        dependencies: Some(vec![
            ResourceFilter::file("/sourceset-a"),
            ResourceFilter::file("/sourceset-b"),
        ]),
    }]);

    assert_eq!(
        fixture.resolve(),
        Err(ResolutionFailure::Ambiguous(
            fixture.prospective_candidates("Target")
        ))
    );
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
    assert_eq!(fixture.resolve(), Ok(fixture.expected("q.Foo")));
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
    assert_eq!(fixture.resolve(), Ok(fixture.expected("p.Foo")));
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
    let Err(ResolutionFailure::Ambiguous(candidates)) = ambiguous.resolve() else {
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
    assert_eq!(duplicate.resolve(), Ok(duplicate.expected("q.Foo")));
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
    let Err(ResolutionFailure::Ambiguous(candidates)) = fixture.resolve() else {
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
    assert_eq!(fixture.resolve(), Ok(fixture.expected("java.lang.String")));
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
    assert_eq!(fixture.resolve(), Ok(fixture.expected("q.Outer.Inner")));

    let missing = Fixture::new(
        "package p; import q.Outer; class Use { Outer.Missing field; }",
        &[("Outer", "package q; public class Outer {}")],
    );
    assert_eq!(missing.resolve(), Err(ResolutionFailure::Partial));
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
    assert_eq!(fixture.resolve(), Ok(fixture.expected("q.Outer.Inner")));
}

#[test]
fn imported_superclass_uses_the_same_external_query() {
    let fixture = Fixture::new(
        "package p; import q.Base; class Use extends Base {}",
        &[("Base", "package q; public class Base {}")],
    );
    assert_eq!(
        fixture.resolve_at(Occurrence::Supertype),
        Ok(fixture.expected("q.Base"))
    );
}

#[test]
fn stored_jvm_classes_are_resolved() {
    let files = TempDir::new();
    let path = files.write("Foo.class", class_bytes("q/Foo"));
    let mut fixture = Fixture::new("package p; import q.Foo; class Use { Foo field; }", &[]);
    fixture.store_jvm_resource(&path);
    assert_eq!(
        fixture.resolve(),
        Ok(TypeCandidate::Jvm(Location::file(path)))
    );
}

#[test]
fn a_jvm_single_import_shadows_a_java_type_in_the_current_package() {
    let files = TempDir::new();
    let path = files.write("Foo.class", class_bytes("q/Foo"));
    let mut fixture = Fixture::new(
        "package p; import q.Foo; class Use { Foo field; }",
        &[("Foo", "package p; class Foo {}")],
    );
    fixture.store_jvm_resource(&path);
    assert_eq!(
        fixture.resolve(),
        Ok(TypeCandidate::Jvm(Location::file(path)))
    );
}

#[test]
fn java_and_jvm_candidates_with_the_same_name_are_ambiguous() {
    let files = TempDir::new();
    let path = files.write("Foo.class", class_bytes("q/Foo"));
    let mut fixture = Fixture::new(
        "package p; import q.Foo; class Use { Foo field; }",
        &[("Foo", "package q; public class Foo {}")],
    );
    fixture.store_jvm_resource(&path);
    assert_eq!(
        fixture.resolve(),
        Err(ResolutionFailure::Ambiguous(vec![
            fixture.expected("q.Foo"),
            TypeCandidate::Jvm(Location::file(path)),
        ]))
    );
}

#[test]
fn an_unmatched_name_is_not_found() {
    let fixture = Fixture::new("package p; class Use { Missing field; }", &[]);
    assert_eq!(fixture.resolve(), Err(ResolutionFailure::NotFound));
}
