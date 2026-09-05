use super::{find_type_body_scope, find_type_declaration};
use crate::lower_into;
use beans_lang_java_model::{File, declarations::types::Kind};

#[test]
fn top_level_declaration_is_entered_in_root_and_introduces_a_body_scope() {
    let file = lower_into("class C {}");
    let root = file.scope(File::ROOT_SCOPE_ID).unwrap();
    let declaration = find_type_declaration(&file, "C");
    let body = find_type_body_scope(&file, declaration.declaration_id);

    assert_eq!(file.iter_scopes().count(), 2);
    assert_eq!(declaration.declaring_scope_id, File::ROOT_SCOPE_ID);
    assert_eq!(
        root.iter_declarations(&file)
            .map(|declaration| declaration.index)
            .collect::<Vec<_>>(),
        [declaration.declaration_id]
    );
    assert_eq!(root.child_scopes(), [body.index]);
    assert_eq!(body.scope.parent_scope(), Some(File::ROOT_SCOPE_ID));
    assert!(body.scope.child_scopes().is_empty());
    assert!(body.scope.iter_declarations(&file).next().is_none());
}

#[test]
fn top_level_declarations_share_a_scope_and_introduce_distinct_body_scopes() {
    let file = lower_into("class First {} class Second {}");
    let root = file.scope(File::ROOT_SCOPE_ID).unwrap();
    let first = find_type_declaration(&file, "First");
    let second = find_type_declaration(&file, "Second");
    let first_body = find_type_body_scope(&file, first.declaration_id);
    let second_body = find_type_body_scope(&file, second.declaration_id);

    assert_eq!(file.iter_scopes().count(), 3);
    assert_eq!(first.declaring_scope_id, File::ROOT_SCOPE_ID);
    assert_eq!(second.declaring_scope_id, File::ROOT_SCOPE_ID);
    assert_ne!(first_body.index, second_body.index);
    assert_eq!(root.child_scopes(), [first_body.index, second_body.index]);
    assert_eq!(
        root.iter_declarations(&file)
            .map(|declaration| declaration.index)
            .collect::<Vec<_>>(),
        [first.declaration_id, second.declaration_id]
    );
    assert_eq!(first_body.scope.parent_scope(), Some(File::ROOT_SCOPE_ID));
    assert_eq!(second_body.scope.parent_scope(), Some(File::ROOT_SCOPE_ID));
}

#[test]
fn member_types_are_entered_in_the_containing_type_body_scope() {
    let file = lower_into("class Outer { class First {} interface Second {} }");
    let root = file.scope(File::ROOT_SCOPE_ID).unwrap();
    let outer = find_type_declaration(&file, "Outer");
    let first = find_type_declaration(&file, "First");
    let second = find_type_declaration(&file, "Second");
    let outer_body = find_type_body_scope(&file, outer.declaration_id);
    let first_body = find_type_body_scope(&file, first.declaration_id);
    let second_body = find_type_body_scope(&file, second.declaration_id);

    assert_eq!(root.child_scopes(), [outer_body.index]);
    assert_eq!(first.declaring_scope_id, outer_body.index);
    assert_eq!(second.declaring_scope_id, outer_body.index);
    assert_eq!(
        outer_body
            .scope
            .iter_declarations(&file)
            .map(|declaration| declaration.index)
            .collect::<Vec<_>>(),
        [first.declaration_id, second.declaration_id]
    );
    assert_eq!(
        outer_body.scope.child_scopes(),
        [first_body.index, second_body.index]
    );
    assert_eq!(first_body.scope.parent_scope(), Some(outer_body.index));
    assert_eq!(second_body.scope.parent_scope(), Some(outer_body.index));
}

#[test]
fn interface_and_annotation_members_use_their_containing_body_scopes() {
    let file = lower_into(
        "interface InterfaceOuter { class InterfaceNested {} }
         @interface AnnotationOuter { class AnnotationNested {} }",
    );
    let interface = find_type_declaration(&file, "InterfaceOuter");
    let interface_nested = find_type_declaration(&file, "InterfaceNested");
    let annotation = find_type_declaration(&file, "AnnotationOuter");
    let annotation_nested = find_type_declaration(&file, "AnnotationNested");
    let interface_body = find_type_body_scope(&file, interface.declaration_id);
    let annotation_body = find_type_body_scope(&file, annotation.declaration_id);

    assert_eq!(interface_nested.declaring_scope_id, interface_body.index);
    assert_eq!(annotation_nested.declaring_scope_id, annotation_body.index);
}

#[test]
fn enum_members_after_constants_use_the_enum_body_scope() {
    let file = lower_into("enum Outer { VALUE; class Nested {} }");
    let outer = find_type_declaration(&file, "Outer");
    let nested = find_type_declaration(&file, "Nested");
    let outer_body = find_type_body_scope(&file, outer.declaration_id);

    assert_eq!(nested.declaring_scope_id, outer_body.index);
    assert_eq!(nested.declaration.kind, Kind::Class);
}
