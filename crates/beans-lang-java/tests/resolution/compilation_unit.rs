use beans_lang_java::{lower_into, semantics::resolution::ResolutionFailure};

use super::{resolve_field, resolved_java_index, type_index};

#[test]
fn a_top_level_type_is_visible_to_another_type_in_the_compilation_unit() {
    let file = lower_into("class Target {} class Use { Target value; }");
    let target = type_index(&file, &["Target"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), target);
}

#[test]
fn a_top_level_type_can_refer_to_itself() {
    let file = lower_into("class Target { Target value; }");
    let target = type_index(&file, &["Target"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), target);
}

#[test]
fn downward_lookup_continues_through_top_level_member_types() {
    let file = lower_into("class Outer { class Inner {} } class Use { Outer.Inner value; }");
    let inner = type_index(&file, &["Outer", "Inner"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), inner);
}

#[test]
fn downward_lookup_includes_inherited_members_of_a_top_level_type() {
    let file = lower_into(
        "class Base { class Member {} } class Child extends Base {} class Use { Child.Member value; }",
    );
    let member = type_index(&file, &["Base", "Member"]);

    let result = resolve_field(&file, "value").unwrap();

    assert_eq!(resolved_java_index(result), member);
}

#[test]
fn duplicate_top_level_types_are_ambiguous() {
    let file = lower_into("class Target {} class Target {} class Use { Target value; }");

    let result = resolve_field(&file, "value");

    let Err(ResolutionFailure::Ambiguous(candidates)) = result else {
        panic!("expected ambiguity, got {result:?}");
    };
    assert_eq!(candidates.len(), 2);
}
