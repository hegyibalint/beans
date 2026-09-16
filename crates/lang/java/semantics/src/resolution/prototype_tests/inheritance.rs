use super::*;

#[test]
fn three_file_example_selects_the_imported_bases_member() {
    let fixture = example();
    let results = fixture.field("src/app/Foo.java");
    assert_eq!(results.len(), 1);
    fixture.assert_type(&results[0], "src/library/Base.java", "library.Base.Bar");
}

#[test]
fn each_supertype_reference_uses_its_owners_imports() {
    let fixture = Fixture::new(&[
        (
            "src/app/Foo.java",
            "package app; import p.Base; import other.Parent; class Foo extends Base { Bar target; }",
        ),
        (
            "src/p/Base.java",
            "package p; import q.Parent; public class Base extends Parent {}",
        ),
        (
            "src/q/Parent.java",
            "package q; public class Parent { public static class Bar {} }",
        ),
        (
            "src/other/Parent.java",
            "package other; public class Parent {}",
        ),
    ]);
    let results = fixture.field("src/app/Foo.java");
    fixture.assert_type(&results[0], "src/q/Parent.java", "q.Parent.Bar");
}

#[test]
fn distinct_inherited_members_are_ambiguous_regardless_of_interface_order() {
    for parents in ["A, B", "B, A"] {
        let text = format!(
            "interface A {{ class Bar {{}} }} interface B {{ class Bar {{}} }} class Use implements {parents} {{ Bar target; }}"
        );
        let fixture = Fixture::new(&[("src/Use.java", &text)]);
        let results = fixture.field("src/Use.java");
        let ResolutionResult::Ambiguous(candidates) = &results[0] else {
            panic!("expected ambiguity: {results:?}");
        };
        assert_eq!(candidates.len(), 2);
        assert_ne!(candidates[0].declaration, candidates[1].declaration);
    }
}

#[test]
fn diamond_paths_to_one_declaration_are_not_ambiguous_or_cyclic() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "interface A { class Bar {} } interface B extends A {} interface C extends A {} class Use implements B, C { Bar target; }",
    )]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/Use.java", "A.Bar");
}

#[test]
fn directly_declared_member_hides_competing_inherited_members() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "interface A { class Bar {} } interface B { class Bar {} } class Use implements A, B { class Bar {} Bar target; }",
    )]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/Use.java", "Use.Bar");
}

#[test]
fn missing_supertype_blocks_fallback_to_a_top_level_name() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Bar {} class Use extends Missing { Bar target; }",
    )]);
    let results = fixture.field("src/Use.java");
    let ResolutionResult::Blocked {
        candidates,
        problems,
    } = &results[0]
    else {
        panic!("expected blocked lookup: {results:?}");
    };
    assert!(candidates.is_empty());
    let [LookupProblem::Supertype { results, .. }] = problems.as_slice() else {
        panic!("expected supertype failure: {problems:?}");
    };
    assert!(matches!(results.as_slice(), [ResolutionResult::NotFound]));
}

#[test]
fn ambiguous_supertype_is_not_explored_but_independent_interfaces_are() {
    let fixture = Fixture::new(&[
        (
            "src/Use.java",
            "import a.Base; import b.Base; interface Good { class Bar {} } class Use extends Base implements Good { Bar target; }",
        ),
        (
            "src/a/Base.java",
            "package a; public class Base { public static class Bar {} }",
        ),
        (
            "src/b/Base.java",
            "package b; public class Base { public static class Bar {} }",
        ),
    ]);
    let results = fixture.field("src/Use.java");
    let ResolutionResult::Blocked {
        candidates,
        problems,
    } = &results[0]
    else {
        panic!("expected blocked lookup: {results:?}");
    };
    assert_eq!(candidates.len(), 1);
    fixture.assert_declaration(&candidates[0].declaration, "src/Use.java", "Good.Bar");
    let [LookupProblem::Supertype { results, .. }] = problems.as_slice() else {
        panic!("expected failed superclass reference: {problems:?}");
    };
    let [ResolutionResult::Ambiguous(bases)] = results.as_slice() else {
        panic!("expected ambiguous Base: {results:?}");
    };
    assert_eq!(bases.len(), 2);
}

#[test]
fn self_and_mutual_inheritance_cycles_are_preserved_without_recursing_forever() {
    for text in [
        "class A extends A { Bar target; }",
        "class A extends B { Bar target; } class B extends A {}",
    ] {
        let fixture = Fixture::new(&[("src/Use.java", text)]);
        let results = fixture.field("src/Use.java");
        let ResolutionResult::Blocked { problems, .. } = &results[0] else {
            panic!("expected cycle: {results:?}");
        };
        assert!(
            problems.iter().any(
                |problem| matches!(problem, LookupProblem::Cycle { name, .. } if name == "Bar")
            )
        );
    }
}

#[test]
fn private_member_is_not_inherited_and_does_not_resurrect_a_hidden_ancestor_member() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Bar {} class Grand { public static class Bar {} } class Base extends Grand { private static class Bar {} } class Use extends Base { Bar target; }",
    )]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/Use.java", "Bar");
}

#[test]
fn package_member_does_not_reappear_after_an_inheritance_edge_crosses_packages() {
    let fixture = Fixture::new(&[
        (
            "src/a/Use.java",
            "package a; import b.Middle; class Bar {} class Use extends Middle { Bar target; }",
        ),
        (
            "src/a/Base.java",
            "package a; public class Base { static class Bar {} }",
        ),
        (
            "src/b/Middle.java",
            "package b; import a.Base; public class Middle extends Base {}",
        ),
    ]);
    let results = fixture.field("src/a/Use.java");
    fixture.assert_type(&results[0], "src/a/Use.java", "a.Bar");
}

#[test]
fn interface_member_types_are_implicitly_public_across_packages() {
    let fixture = Fixture::new(&[
        (
            "src/Use.java",
            "import library.Base; class Use implements Base { Bar target; }",
        ),
        (
            "src/library/Base.java",
            "package library; public interface Base { class Bar {} }",
        ),
    ]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/library/Base.java", "library.Base.Bar");
}

#[test]
fn private_member_is_accessible_within_its_top_level_nest() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Outer { private class Hidden {} class Inner { Outer.Hidden target; } }",
    )]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[1], "src/Use.java", "Outer.Hidden");
}

#[test]
fn inaccessible_member_is_not_reported_as_missing_or_replaced_by_an_outer_name() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Hidden {} class Outer { private class Hidden {} } class Use { Outer.Hidden target; }",
    )]);
    let results = fixture.field("src/Use.java");
    let ResolutionResult::Blocked {
        candidates,
        problems,
    } = &results[1]
    else {
        panic!("expected inaccessible member: {results:?}");
    };
    assert_eq!(candidates.len(), 1);
    assert!(matches!(
        problems.as_slice(),
        [LookupProblem::Inaccessible(_)]
    ));
}
