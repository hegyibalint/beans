use super::{ResolutionResult, lookup_field};
use crate::resolution::{ResolvedDeclarationHandle, ResolvedTypeParameter};
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
        lookup_field(
            &format!("class Outer {{ {members} int target; }}"),
            |instance, entry| {
                let result = instance.lookup_simple_type_declaration(entry, "A");
                let ResolutionResult::Resolved(ResolvedTypeParameter::Type(target)) = &result
                else {
                    panic!("expected one declaration");
                };
                let ResolvedDeclarationHandle::Java(handle) = &target.declaration else {
                    panic!("expected a Java declaration");
                };
                let found = instance.query.declaration(handle).unwrap();
                assert_eq!(found.source, instance.source);
                assert!(std::ptr::eq(found.file, instance.file));
                assert_eq!(found.declaration.name.as_deref(), Some("A"));
                assert_eq!(
                    found.file.node(found.node_index).unwrap().parent(),
                    Some(entry.index)
                );
                assert!(target.arguments.is_empty());
                result
            },
        );
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

    assert!(matches!(result, ResolutionResult::Resolved(_)));
}

#[test]
fn duplicate_declarations_produce_ambiguity_during_error_recovery() {
    for members in [
        "class A {} class A {}",
        "class A {} class B {} class A {} class A {}",
    ] {
        lookup_field(
            &format!("class Outer {{ {members} int target; }}"),
            |instance, entry| {
                let result = instance.lookup_simple_type_declaration(entry, "A");
                let ResolutionResult::Ambiguous(targets) = &result else {
                    panic!("expected all ambiguous declarations");
                };
                let expected: Vec<_> = instance
                    .file
                    .iter_children(entry.index)
                    .filter(|child| {
                        child
                            .node
                            .kind()
                            .as_type()
                            .is_some_and(|typ| typ.name.as_deref() == Some("A"))
                    })
                    .map(|child| {
                        instance
                            .query
                            .declaration_handle(instance.source, child.index)
                    })
                    .collect();
                assert_eq!(targets.len(), expected.len());
                for (target, expected) in targets.iter().zip(expected) {
                    assert_eq!(
                        target.declaration,
                        ResolvedDeclarationHandle::Java(expected)
                    );
                    assert!(target.arguments.is_empty());
                }
                result
            },
        );
    }
}
