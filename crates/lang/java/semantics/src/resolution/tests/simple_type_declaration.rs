use super::{ResolutionResult, lookup_field};
use beans_lang_java_model::File;

fn lookup(source: &str, name: &str) -> ResolutionResult {
    lookup_field(source, |instance, entry| {
        instance.lookup_simple_type_declaration(entry, name)
    })
}

#[test]
fn body_without_type_declarations_is_not_found() {
    assert!(matches!(
        lookup("class Outer { int target; }", "A"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn matching_member_is_found_among_unrelated_types_regardless_of_order() {
    for members in ["class A {} class B {}", "class B {} class A {}"] {
        assert!(matches!(
            lookup(&format!("class Outer {{ {members} int target; }}"), "A"),
            ResolutionResult::Resolved
        ));
    }
}

#[test]
fn unrelated_type_names_are_not_found() {
    assert!(matches!(
        lookup("class Outer { class B {} int target; }", "A"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn same_named_fields_are_not_type_candidates() {
    assert!(matches!(
        lookup("class Outer { int A; int target; }", "A"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn owners_parameters_are_not_type_declarations_in_the_body() {
    assert!(matches!(
        lookup("class Outer<A> { int target; }", "A"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn enclosing_scopes_are_not_searched() {
    assert!(matches!(
        lookup(
            "class Outer { class A {} class Inner { int target; } }",
            "A"
        ),
        ResolutionResult::NotFound
    ));
}

#[test]
fn nested_bodies_are_not_searched() {
    assert!(matches!(
        lookup(
            "class Outer { class Nested { class A {} } int target; }",
            "A"
        ),
        ResolutionResult::NotFound
    ));
}

#[test]
fn supplied_root_scope_is_searched_instead_of_the_occurrence_scope() {
    let result = lookup_field("class A {} class Outer { int target; }", |instance, _| {
        let root = instance
            .file
            .iter_ancestors(File::ROOT_NODE_ID)
            .next()
            .unwrap();
        instance.lookup_simple_type_declaration(root, "A")
    });

    assert!(matches!(result, ResolutionResult::Resolved));
}

#[test]
fn duplicate_declarations_produce_ambiguity_during_error_recovery() {
    assert!(matches!(
        lookup("class Outer { class A {} class A {} int target; }", "A"),
        ResolutionResult::Ambigous
    ));
}
