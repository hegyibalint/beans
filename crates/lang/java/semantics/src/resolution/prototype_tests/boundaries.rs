use super::*;

#[test]
fn unavailable_occurrence_model_is_reported_without_panicking() {
    let fixture = Fixture::new(&[]);
    let query = fixture.query();
    let reference = TypeRef::Named {
        segments: vec![TypeNameComponent {
            name: "Missing".into(),
            bounds: Vec::new(),
        }],
    };
    let source = source("src/Missing.java");
    let results = Resolver {}.resolve(&ResolverContext::new(
        &source,
        File::ROOT_NODE_ID,
        &reference,
        ReferenceLocation::Body,
        &query,
    ));
    assert!(
        matches!(results.as_slice(), [ResolutionResult::Blocked { problems, .. }]
        if matches!(problems.as_slice(), [LookupProblem::UnavailableModel]))
    );
}

#[test]
fn unsupported_import_forms_are_not_silently_treated_as_absent() {
    for import in [
        "import p.*;",
        "import static p.Outer.Bar;",
        "import static p.Outer.*;",
    ] {
        let text = format!("{import} class Use {{ Bar target; }}");
        let fixture = Fixture::new(&[("src/Use.java", &text)]);
        let results = fixture.field("src/Use.java");
        assert!(
            matches!(&results[0], ResolutionResult::Blocked { problems, .. }
            if problems.iter().any(|problem| matches!(problem, LookupProblem::Unsupported(_))))
        );
    }
}

#[test]
#[should_panic(expected = "member lookup is not implemented for")]
fn parameter_member_lookup_panics_until_it_is_implemented() {
    let fixture = Fixture::new(&[("src/Use.java", "class Use<T> { T.Bar target; }")]);
    fixture.field("src/Use.java");
}

#[test]
fn cross_package_protected_access_keeps_the_candidate_until_the_rule_is_implemented() {
    let fixture = Fixture::new(&[
        (
            "src/Use.java",
            "import p.Base; class Use extends Base { Bar target; }",
        ),
        (
            "src/p/Base.java",
            "package p; public class Base { protected static class Bar {} }",
        ),
    ]);
    let results = fixture.field("src/Use.java");
    let ResolutionResult::Blocked {
        candidates,
        problems,
    } = &results[0]
    else {
        panic!("expected explicit unsupported access: {results:?}");
    };
    assert_eq!(candidates.len(), 1);
    assert!(matches!(
        problems.as_slice(),
        [LookupProblem::Unsupported(_)]
    ));
}

#[test]
fn a_bound_uses_its_owners_parameters_but_not_its_body_members() {
    let fixture = Fixture::new(&[("src/Use.java", "class Use<T extends U, U> { class U {} }")]);
    let source = source("src/Use.java");
    let query = fixture.query();
    let file = query.file(&source).unwrap();
    let (node, declaration) = file.find_type(&name("Use")).pop().unwrap();
    let reference = declaration.type_parameters[0].bounds[0].primary().unwrap();
    let results = Resolver {}.resolve(&ResolverContext::new(
        &source,
        node,
        reference,
        ReferenceLocation::TypeParameterBound,
        &query,
    ));
    assert_parameter(&results[0], "U");
}

#[test]
fn supertype_name_collision_with_an_owner_parameter_is_not_silently_rebound() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Base {} class Use<Base> extends Base {}",
    )]);
    let results = fixture.superclass("src/Use.java", "Use");
    assert!(
        matches!(&results[0], ResolutionResult::Blocked { problems, .. }
        if matches!(problems.as_slice(), [LookupProblem::Unsupported(_)]))
    );
}

#[test]
fn unknown_implicit_supertype_does_not_disappear_beside_a_known_interface_member() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "interface I { class Bar {} } enum Use implements I { VALUE; Bar target; }",
    )]);
    let results = fixture.field("src/Use.java");
    let ResolutionResult::Blocked {
        candidates,
        problems,
    } = &results[0]
    else {
        panic!("expected unresolved implicit supertype: {results:?}");
    };
    assert_eq!(candidates.len(), 1);
    assert!(matches!(
        problems.as_slice(),
        [LookupProblem::Unsupported(_)]
    ));
}

#[test]
fn resolved_handles_remain_readable_at_their_original_revision() {
    let mut fixture = example();
    let results = fixture.field("src/app/Foo.java");
    fixture.files.put(
        Revision::new(2),
        source("src/library/Base.java"),
        crate::lower_into("package library; public class Base {}"),
    );
    let query = JavaQuery::new(&fixture.files, Revision::new(2), &fixture.classpath);
    let ResolvedDeclaration::Java { declaration } = &resolved(&results[0]).declaration else {
        panic!("expected Java declaration");
    };
    assert_eq!(
        query
            .declaration(declaration)
            .unwrap()
            .declaration
            .name
            .as_deref(),
        Some("Bar")
    );
}
