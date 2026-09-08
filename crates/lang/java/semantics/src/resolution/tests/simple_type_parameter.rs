use super::{lookup_field, ResolutionResult};
use beans_lang_java_model::File;

fn lookup(source: &str, name: &str) -> ResolutionResult {
    lookup_field(source, |instance, entry| {
        instance.lookup_simple_type_parameter(entry, name)
    })
}

#[test]
fn owners_parameter_is_found() {
    assert!(matches!(
        lookup("class Outer<A> { class Inner<B> { int target; } }", "B"),
        ResolutionResult::Resolved
    ));
}

#[test]
fn owner_without_parameters_is_not_found() {
    assert!(matches!(
        lookup("class Outer { int target; }", "A"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn enclosing_owners_parameters_are_not_searched() {
    assert!(matches!(
        lookup("class Outer<A> { class Inner<B> { int target; } }", "A"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn nested_types_parameters_are_not_searched() {
    assert!(matches!(
        lookup("class Outer { class Inner<B> {} int target; }", "B"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn siblings_parameters_are_not_searched() {
    assert!(matches!(
        lookup(
            "class Outer { class Sibling<B> {} class Inner { int target; } }",
            "B"
        ),
        ResolutionResult::NotFound
    ));
}

#[test]
fn member_type_names_are_not_parameter_candidates() {
    assert!(matches!(
        lookup("class Outer { class A {} int target; }", "A"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn supplied_owner_is_used_instead_of_the_occurrence_owner() {
    let result = lookup_field(
        "class Outer<A> { class Inner<B> { int target; } }",
        |instance, entry| {
            let outer = instance
                .file
                .iter_scopes_from(entry.scope_index)
                .nth(1)
                .unwrap();
            instance.lookup_simple_type_parameter(outer, "A")
        },
    );

    assert!(matches!(result, ResolutionResult::Resolved));
}

#[test]
fn root_scope_has_no_owner_parameters_even_when_the_occurrence_does() {
    let result = lookup_field("class Outer<A> { int target; }", |instance, _| {
        let root = instance
            .file
            .iter_scopes_from(File::ROOT_SCOPE_ID)
            .next()
            .unwrap();
        instance.lookup_simple_type_parameter(root, "A")
    });

    assert!(matches!(result, ResolutionResult::NotFound));
}
