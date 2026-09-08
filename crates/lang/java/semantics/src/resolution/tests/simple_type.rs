use super::{lookup_field, ResolutionResult};
use crate::resolution::ResolutionInstance;
use beans_lang_java_model::references::TypeRef;

fn lookup(source: &str) -> ResolutionResult {
    lookup_field(source, |instance, entry| instance.lookup_simple_type(entry))
}

#[test]
fn declaration_stage_is_connected() {
    assert!(matches!(
        lookup("class Outer { class A {} A target; }"),
        ResolutionResult::Resolved
    ));
}

#[test]
fn missing_declaration_falls_through_to_parameter_stage() {
    assert!(matches!(
        lookup("class Outer<A> { A target; }"),
        ResolutionResult::Resolved
    ));
}

#[test]
fn missing_name_exhausts_both_stages() {
    assert!(matches!(
        lookup("class Outer { Missing target; }"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn declaration_ambiguity_is_not_rescued_by_a_parameter_during_error_recovery() {
    assert!(matches!(
        lookup("class Outer<A> { class A {} class A {} A target; }"),
        ResolutionResult::Ambigous
    ));
}

#[test]
fn qualified_names_are_not_treated_as_either_simple_component() {
    assert!(matches!(
        lookup("class Outer { class A { class B {} } class B {} A.B target; }"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn arrays_are_not_resolved_as_their_element_name() {
    assert!(matches!(
        lookup("class Outer<A> { A[] target; }"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn primitive_references_do_not_enter_name_lookup() {
    assert!(matches!(
        lookup("class Outer { int target; }"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn void_and_empty_named_references_do_not_enter_name_lookup() {
    for type_ref in [TypeRef::Void, TypeRef::Named { segments: vec![] }] {
        let result = lookup_field("class Outer { int target; }", |instance, entry| {
            ResolutionInstance::new(
                instance.resolver,
                instance.file,
                instance.scope_index,
                &type_ref,
                instance.classpath,
            )
            .lookup_simple_type(entry)
        });
        assert!(matches!(result, ResolutionResult::NotFound));
    }
}
