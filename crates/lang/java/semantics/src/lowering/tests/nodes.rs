use super::find_type_declaration;
use crate::lower_into;
use beans_lang_java_model::{
    File,
    nodes::{NodeKind, types::Kind},
};

#[test]
fn a_type_is_one_node_containing_its_members_without_a_separate_body_node() {
    let file = lower_into("class Outer { int first; class Member { int nested; } int last; }");
    let outer = find_type_declaration(&file, "Outer");
    let member = find_type_declaration(&file, "Member");
    assert_eq!(file.iter_nodes().count(), 6);
    assert_eq!(outer.parent, File::ROOT_NODE_ID);
    assert_eq!(member.parent, outer.index);
    assert_eq!(
        file.iter_children(File::ROOT_NODE_ID)
            .map(|entry| entry.index)
            .collect::<Vec<_>>(),
        [outer.index]
    );
    let children: Vec<_> = file.iter_children(outer.index).collect();
    let [first, nested, last] = children.as_slice() else {
        panic!("expected three children");
    };
    assert!(matches!(first.node.kind(), NodeKind::Field(field) if field.name == "first"));
    assert_eq!(nested.index, member.index);
    assert!(matches!(last.node.kind(), NodeKind::Field(field) if field.name == "last"));
    assert!(
        matches!(file.iter_children(member.index).next().unwrap().node.kind(), NodeKind::Field(field) if field.name == "nested")
    );
}

#[test]
fn sibling_types_keep_their_members_under_their_own_nodes() {
    let file = lower_into("class First { class Member {} } class Second { class Member {} }");
    let first = find_type_declaration(&file, "First");
    let second = find_type_declaration(&file, "Second");
    assert_eq!(
        file.iter_children(File::ROOT_NODE_ID)
            .map(|entry| entry.index)
            .collect::<Vec<_>>(),
        [first.index, second.index]
    );
    let first_member = file.iter_children(first.index).next().unwrap();
    let second_member = file.iter_children(second.index).next().unwrap();
    assert_ne!(first_member.index, second_member.index);
    assert_eq!(first_member.node.parent(), Some(first.index));
    assert_eq!(second_member.node.parent(), Some(second.index));
}

#[test]
fn interface_and_annotation_members_are_children_of_their_containing_types() {
    let file = lower_into(
        "interface InterfaceOuter { class InterfaceNested {} }
         @interface AnnotationOuter { class AnnotationNested {} }",
    );
    let interface = find_type_declaration(&file, "InterfaceOuter");
    let interface_nested = find_type_declaration(&file, "InterfaceNested");
    let annotation = find_type_declaration(&file, "AnnotationOuter");
    let annotation_nested = find_type_declaration(&file, "AnnotationNested");
    assert_eq!(interface_nested.parent, interface.index);
    assert_eq!(annotation_nested.parent, annotation.index);
}

#[test]
fn enum_members_after_constants_are_children_of_the_enum() {
    let file = lower_into("enum Outer { VALUE; class Nested {} }");
    let outer = find_type_declaration(&file, "Outer");
    let nested = find_type_declaration(&file, "Nested");
    assert_eq!(nested.parent, outer.index);
    assert_eq!(nested.declaration.kind, Kind::Class);
}

#[test]
fn record_members_are_children_of_the_record() {
    let file = lower_into("record Outer(int value) { class Nested {} }");
    let outer = find_type_declaration(&file, "Outer");
    let nested = find_type_declaration(&file, "Nested");
    assert_eq!(nested.parent, outer.index);
}
