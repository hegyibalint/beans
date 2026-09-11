use super::{ResolutionResult, lookup_field};

fn resolve(source: &str) -> ResolutionResult {
    lookup_field(source, |instance, _| instance.resolve())
}

#[test]
fn not_found_walks_outward_to_an_enclosing_owner() {
    assert!(matches!(
        resolve("class Outer<A> { class Middle { class Inner { A target; } } }"),
        ResolutionResult::Resolved
    ));
}

#[test]
fn not_found_walks_all_the_way_to_the_compilation_unit() {
    assert!(matches!(
        resolve("class A {} class Outer { class Inner { A target; } }"),
        ResolutionResult::Resolved
    ));
}

#[test]
fn resolved_inner_scope_stops_before_an_ambiguous_outer_scope() {
    assert!(matches!(
        resolve("class Outer { class A {} class A {} class Inner { class A {} A target; } }"),
        ResolutionResult::Resolved
    ));
}

#[test]
fn ambiguous_inner_scope_is_not_rescued_by_an_enclosing_parameter() {
    assert!(matches!(
        resolve("class Outer<A> { class Inner { class A {} class A {} A target; } }"),
        ResolutionResult::Ambigous
    ));
}

#[test]
fn exhausted_scopes_are_not_found() {
    assert!(matches!(
        resolve("class Outer<A> { class Inner<B> { Missing target; } }"),
        ResolutionResult::NotFound
    ));
}

#[test]
fn public_resolver_forwards_the_occurrence_context() {
    let result = lookup_field(
        "class Outer<A> { class Inner { A target; } }",
        |instance, _| {
            instance.resolver.resolve(
                instance.file,
                instance.node_index,
                instance.type_ref,
                instance.query,
            )
        },
    );

    assert!(matches!(result, ResolutionResult::Resolved));
}
