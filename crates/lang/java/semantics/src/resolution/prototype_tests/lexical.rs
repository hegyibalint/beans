use super::*;

#[test]
fn directly_declared_members_shadow_owner_parameters_inherited_members_and_imports() {
    let fixture = Fixture::new(&[
        (
            "src/Use.java",
            "import other.T; class Base { static class T {} } class Use<T> extends Base { class T {} T target; }",
        ),
        ("src/other/T.java", "package other; public class T {}"),
    ]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/Use.java", "Use.T");
}

#[test]
fn owner_parameter_shadows_inherited_member() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Base { static class T {} } class Use<T> extends Base { T target; }",
    )]);
    let results = fixture.field("src/Use.java");
    let owner = assert_parameter(&results[0], "T");
    let query = fixture.query();
    assert_eq!(
        query
            .declaration(owner)
            .unwrap()
            .declaration
            .name
            .as_deref(),
        Some("Use")
    );
}

#[test]
fn inherited_member_shadows_enclosing_type_parameter() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Base { static class T {} } class Outer<T> { class Inner extends Base { T target; } }",
    )]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/Use.java", "Base.T");
}

#[test]
fn own_parameter_does_not_require_resolving_a_broken_superclass() {
    let fixture = Fixture::new(&[("src/Use.java", "class Use<T> extends Missing { T target; }")]);
    let results = fixture.field("src/Use.java");
    assert_parameter(&results[0], "T");
}

#[test]
fn enclosing_member_precedes_compilation_unit_declaration() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Bar {} class Outer { class Bar {} class Inner { Bar target; } }",
    )]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/Use.java", "Outer.Bar");
}

#[test]
fn compilation_unit_and_same_package_are_distinct_lookup_layers() {
    let fixture = Fixture::new(&[
        ("src/Use.java", "package app; class Use { Bar target; }"),
        ("src/Bar.java", "package app; class Bar {}"),
    ]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/Bar.java", "app.Bar");
}

#[test]
fn qualified_member_lookup_does_not_fall_back_to_enclosing_scopes() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Use { class Bar {} class Outer {} Outer.Bar target; }",
    )]);
    let results = fixture.field("src/Use.java");
    assert_eq!(results.len(), 2);
    fixture.assert_type(&results[0], "src/Use.java", "Use.Outer");
    assert!(matches!(results[1], ResolutionResult::NotFound));
}

#[test]
fn a_type_parameter_is_not_a_member_of_its_owner() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Outer<T> {} class Use { Outer.T target; }",
    )]);
    let results = fixture.field("src/Use.java");
    assert_eq!(results.len(), 2);
    assert!(matches!(results[1], ResolutionResult::NotFound));
}

#[test]
fn duplicate_member_declarations_remain_distinct_and_stop_the_qualified_chain() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Outer { class Bar {} class Bar {} } class Use { Outer.Bar.Next target; }",
    )]);
    let results = fixture.field("src/Use.java");
    assert_eq!(results.len(), 2);
    let ResolutionResult::Ambiguous(candidates) = &results[1] else {
        panic!("expected ambiguity: {results:?}");
    };
    assert_eq!(candidates.len(), 2);
    assert_ne!(candidates[0].declaration, candidates[1].declaration);
}

#[test]
fn superclass_lookup_excludes_the_owners_body_members() {
    let fixture = Fixture::new(&[
        (
            "src/Use.java",
            "import library.Base; class Use extends Base { class Base {} }",
        ),
        (
            "src/library/Base.java",
            "package library; public class Base {}",
        ),
    ]);
    let results = fixture.superclass("src/Use.java", "Use");
    fixture.assert_type(&results[0], "src/library/Base.java", "library.Base");
}

#[test]
fn superclass_lookup_still_sees_members_of_an_enclosing_type() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Outer { class Base {} class Use extends Base {} }",
    )]);
    let results = fixture.superclass("src/Use.java", "Use");
    fixture.assert_type(&results[0], "src/Use.java", "Outer.Base");
}
