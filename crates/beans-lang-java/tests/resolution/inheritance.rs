use beans_lang_java::{lower_into, semantics::resolution::ResolutionFailure};

use super::{resolve_field, resolved_java_index, type_index};

#[test]
fn member_types_are_inherited_transitively() {
    let file = lower_into(
        "class Scope { class Grandparent { class Member {} } class Parent extends Grandparent {} class Child extends Parent { Member target; } }",
    );
    let member = type_index(&file, &["Scope", "Grandparent", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn the_same_declaration_reached_through_a_diamond_is_not_ambiguous() {
    let file = lower_into(
        "class Scope { interface Root { class Member {} } interface Left extends Root {} interface Right extends Root {} class Child implements Left, Right { Member target; } }",
    );
    let member = type_index(&file, &["Scope", "Root", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn distinct_declarations_inherited_from_different_interfaces_are_ambiguous() {
    let file = lower_into(
        "class Scope { interface Left { class Member {} } interface Right { class Member {} } class Child implements Left, Right { Member target; } }",
    );

    let result = resolve_field(&file, "target");

    let Err(ResolutionFailure::Ambiguous(candidates)) = result else {
        panic!("expected ambiguity, got {result:?}");
    };
    let indices: Vec<_> = candidates.into_iter().map(resolved_java_index).collect();
    assert_eq!(
        indices,
        [
            type_index(&file, &["Scope", "Left", "Member"]),
            type_index(&file, &["Scope", "Right", "Member"]),
        ]
    );
}

#[test]
fn a_private_declaration_hides_its_ancestors_without_being_inherited() {
    let file = lower_into(
        "class Scope { class Ancestor { class Member {} } class Parent extends Ancestor { private class Member {} } class Child extends Parent { Member target; } }",
    );

    let result = resolve_field(&file, "target");

    assert_eq!(result, Err(ResolutionFailure::NotFound));
}

#[test]
fn inherited_members_win_before_the_next_enclosing_type() {
    let file = lower_into(
        "class Scope { class Member {} class Base { class Member {} } class Child extends Base { Member target; } }",
    );
    let inherited = type_index(&file, &["Scope", "Base", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), inherited);
}

#[test]
fn qualified_paths_include_inherited_member_types() {
    let file = lower_into(
        "class Scope { class Base { class Member {} } class Child extends Base {} class Use { Child.Member target; } }",
    );
    let inherited = type_index(&file, &["Scope", "Base", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), inherited);
}

#[test]
fn an_unresolved_supertype_does_not_hide_later_interface_results() {
    let file = lower_into(
        "class Scope { interface Contract { class Member {} } class Child extends Missing implements Contract { Member target; } }",
    );
    let member = type_index(&file, &["Scope", "Contract", "Member"]);

    let result = resolve_field(&file, "target").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn circular_inheritance_terminates_without_producing_a_candidate() {
    let file = lower_into(
        "class Scope { class First extends Second { Missing target; } class Second extends First {} }",
    );

    let result = resolve_field(&file, "target");

    assert_eq!(result, Err(ResolutionFailure::NotFound));
}

#[test]
fn ambiguity_candidates_follow_superclass_then_interface_order() {
    let file = lower_into(
        "class Scope {
            class Base { class Member {} }
            interface First { class Member {} }
            interface Second { class Member {} }
            class Child extends Base implements First, Second { Member target; }
        }",
    );
    let Err(ResolutionFailure::Ambiguous(candidates)) = resolve_field(&file, "target") else {
        panic!("expected ambiguous inherited members");
    };

    assert_eq!(
        candidates
            .into_iter()
            .map(resolved_java_index)
            .collect::<Vec<_>>(),
        ["Base", "First", "Second"].map(|owner| type_index(&file, &["Scope", owner, "Member"]))
    );
}
