use super::{ResolutionInstance, ResolutionResult, lookup_field};
use crate::resolution::{ResolvedDeclarationHandle, ResolvedTypeParameter};
use beans_lang_java_model::{File, NodeEntry, nodes::NodeKind};

fn assert_parameter(
    instance: &ResolutionInstance<'_>,
    expected_owner: NodeEntry<'_>,
    expected_name: &str,
    result: &ResolutionResult,
) {
    let ResolutionResult::Resolved(ResolvedTypeParameter::Parameter { owner, name }) = result
    else {
        panic!("expected a resolved parameter, got {result:?}");
    };
    assert_eq!(name, expected_name);
    let ResolvedDeclarationHandle::Java(handle) = owner else {
        panic!("expected a Java owner");
    };
    let found = instance.query.declaration(handle).unwrap();
    assert_eq!(found.source, instance.source);
    assert!(std::ptr::eq(found.file, instance.file));
    assert_eq!(found.node_index, expected_owner.index);
    assert!(found.declaration.type_parameter_named(name).is_some());
}

fn lookup(source: &str, name: &str) -> ResolutionResult {
    lookup_field(source, |instance, entry| {
        instance.lookup_simple_type_parameter(entry, name)
    })
}

#[test]
fn owners_parameter_is_found() {
    for name in ["B", "C", "D"] {
        lookup_field(
            "class Outer<A> { class Inner<B, C, D> { int target; } }",
            |instance, entry| {
                let result = instance.lookup_simple_type_parameter(entry, name);
                assert_parameter(instance, entry, name, &result);
                result
            },
        );
    }
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
            let outer = instance.file.iter_ancestors(entry.index).nth(1).unwrap();
            let result = instance.lookup_simple_type_parameter(outer, "A");
            assert_parameter(instance, outer, "A", &result);
            result
        },
    );

    assert!(matches!(
        result,
        ResolutionResult::Resolved(ResolvedTypeParameter::Parameter { .. })
    ));
}

#[test]
fn parameter_names_are_matched_exactly() {
    for name in ["a", "B", ""] {
        assert!(matches!(
            lookup("class Outer<A> { int target; }", name),
            ResolutionResult::NotFound
        ));
    }
}

#[test]
fn recursive_bounds_are_not_resolved_or_used_as_parameter_identity() {
    lookup_field(
        "class Outer<A extends Comparable<A>> { int target; }",
        |instance, entry| {
            let result = instance.lookup_simple_type_parameter(entry, "A");
            assert_parameter(instance, entry, "A", &result);
            result
        },
    );
}

#[test]
fn same_named_parameters_on_different_owners_have_distinct_identities() {
    lookup_field(
        "class Outer<A> { class Inner<A> { int target; } }",
        |instance, inner| {
            let outer = instance.file.iter_ancestors(inner.index).nth(1).unwrap();
            let inner_result = instance.lookup_simple_type_parameter(inner, "A");
            let outer_result = instance.lookup_simple_type_parameter(outer, "A");
            assert_parameter(instance, inner, "A", &inner_result);
            assert_parameter(instance, outer, "A", &outer_result);
            let ResolutionResult::Resolved(ResolvedTypeParameter::Parameter {
                owner: inner_owner,
                ..
            }) = &inner_result
            else {
                unreachable!()
            };
            let ResolutionResult::Resolved(ResolvedTypeParameter::Parameter {
                owner: outer_owner,
                ..
            }) = &outer_result
            else {
                unreachable!()
            };
            assert_ne!(inner_owner, outer_owner);
            inner_result
        },
    );
}

#[test]
fn field_nodes_do_not_expose_their_owners_parameters() {
    lookup_field("class Outer<A> { int target; }", |instance, _| {
        let field = instance
            .file
            .iter_nodes()
            .find(|entry| matches!(entry.node.kind(), NodeKind::Field(_)))
            .unwrap();
        let result = instance.lookup_simple_type_parameter(field, "A");
        assert!(matches!(result, ResolutionResult::NotFound));
        result
    });
}

#[test]
fn root_scope_has_no_owner_parameters_even_when_the_occurrence_does() {
    let result = lookup_field("class Outer<A> { int target; }", |instance, _| {
        let root = instance
            .file
            .iter_ancestors(File::ROOT_NODE_ID)
            .next()
            .unwrap();
        instance.lookup_simple_type_parameter(root, "A")
    });

    assert!(matches!(result, ResolutionResult::NotFound));
}
