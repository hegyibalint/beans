use super::*;
use beans_lang_java_model::references::BoundKind;

#[test]
fn superclass_arguments_see_the_owners_type_parameters() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Base<T> {} class Use<T> extends Base<T> {}",
    )]);
    let results = fixture.superclass("src/Use.java", "Use");
    fixture.assert_type(&results[0], "src/Use.java", "Base");
    let arguments = &resolved(&results[0]).arguments;
    assert_eq!(arguments.len(), 1);
    let owner = assert_parameter(&arguments[0].primary().unwrap()[0], "T");
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
fn nested_superclass_arguments_keep_the_same_reference_location() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Base<T> {} class Box<T> {} class Use<T> extends Base<Box<T>> { class Box {} }",
    )]);
    let results = fixture.superclass("src/Use.java", "Use");
    let box_result = &resolved(&results[0]).arguments[0].primary().unwrap()[0];
    fixture.assert_type(box_result, "src/Use.java", "Box");
    let t_result = &resolved(box_result).arguments[0].primary().unwrap()[0];
    assert_parameter(t_result, "T");
}

#[test]
fn qualified_component_arguments_stay_at_the_original_use_site() {
    let fixture = Fixture::new(&[
        (
            "src/Use.java",
            "import p.Outer; class A {} class Use<B> { Outer<A>.Inner<B> target; }",
        ),
        (
            "src/p/Outer.java",
            "package p; public class Outer<B> { public class Inner<C> {} }",
        ),
    ]);
    let results = fixture.field("src/Use.java");
    assert_eq!(results.len(), 2);
    fixture.assert_type(&results[0], "src/p/Outer.java", "p.Outer");
    fixture.assert_type(&results[1], "src/p/Outer.java", "p.Outer.Inner");
    fixture.assert_type(
        &resolved(&results[0]).arguments[0].primary().unwrap()[0],
        "src/Use.java",
        "A",
    );
    let owner = assert_parameter(
        &resolved(&results[1]).arguments[0].primary().unwrap()[0],
        "B",
    );
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
fn argument_failure_does_not_prevent_resolving_the_outer_declaration_or_its_members() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class Outer<T> { class Inner {} } class Use { Outer<Missing>.Inner target; }",
    )]);
    let results = fixture.field("src/Use.java");
    assert_eq!(results.len(), 2);
    fixture.assert_type(&results[1], "src/Use.java", "Outer.Inner");
    assert!(matches!(
        resolved(&results[0]).arguments[0]
            .primary()
            .unwrap()
            .as_slice(),
        [ResolutionResult::NotFound]
    ));
}

#[test]
fn argument_ambiguity_stays_inside_the_argument() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class A {} class A {} class Box<T> {} class Use { Box<A> target; }",
    )]);
    let results = fixture.field("src/Use.java");
    fixture.assert_type(&results[0], "src/Use.java", "Box");
    assert!(
        matches!(resolved(&results[0]).arguments[0].primary().unwrap().as_slice(), [ResolutionResult::Ambiguous(candidates)] if candidates.len() == 2)
    );
}

#[test]
fn exact_and_wildcard_shapes_survive_real_reference_resolution() {
    let fixture = Fixture::new(&[(
        "src/Use.java",
        "class A {} class Box<W,X,Y,Z> {} class Use { Box<A, ? extends A, ? super A, ?> target; }",
    )]);
    let results = fixture.field("src/Use.java");
    let arguments = &resolved(&results[0]).arguments;
    assert_eq!(
        arguments.iter().map(TypeBound::kind).collect::<Vec<_>>(),
        [
            BoundKind::Exact,
            BoundKind::Extends,
            BoundKind::Super,
            BoundKind::Unbounded
        ]
    );
    for argument in &arguments[..3] {
        fixture.assert_type(&argument.primary().unwrap()[0], "src/Use.java", "A");
    }
    assert!(arguments[3].types().is_empty());
}
