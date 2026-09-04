use super::find_type_declaration;
use crate::lower_into;
use beans_lang_java_model::{File, declarations::types::Kind};

#[test]
fn one_top_level_declaration_has_its_own_scope() {
    let file = lower_into("class C {}");
    let root = file.scope(File::ROOT_SCOPE_ID).unwrap();
    let declaration = find_type_declaration(&file, "C");

    assert_eq!(file.iter_scopes().count(), 2);
    assert_ne!(declaration.scope_id, File::ROOT_SCOPE_ID);
    assert_eq!(root.child_scopes(), [declaration.scope_id]);
    assert!(root.iter_declarations(&file).next().is_none());
    assert_eq!(declaration.scope.parent_scope(), Some(File::ROOT_SCOPE_ID));
    assert!(declaration.scope.child_scopes().is_empty());
    assert_eq!(
        declaration
            .scope
            .iter_declarations(&file)
            .map(|indexed_declaration| indexed_declaration.index)
            .collect::<Vec<_>>(),
        [declaration.declaration_id]
    );
}

#[test]
fn two_top_level_declarations_have_distinct_sibling_scopes() {
    let file = lower_into("class First {} class Second {}");
    let root = file.scope(File::ROOT_SCOPE_ID).unwrap();
    let first = find_type_declaration(&file, "First");
    let second = find_type_declaration(&file, "Second");

    assert_eq!(file.iter_scopes().count(), 3);
    assert_ne!(first.scope_id, File::ROOT_SCOPE_ID);
    assert_ne!(second.scope_id, File::ROOT_SCOPE_ID);
    assert_ne!(first.scope_id, second.scope_id);
    assert_eq!(root.child_scopes(), [first.scope_id, second.scope_id]);
    assert!(root.iter_declarations(&file).next().is_none());
    assert_eq!(first.scope.parent_scope(), Some(File::ROOT_SCOPE_ID));
    assert_eq!(second.scope.parent_scope(), Some(File::ROOT_SCOPE_ID));
}

#[test]
fn nested_declaration_scopes_have_the_containing_type_scope_as_parent() {
    let file = lower_into("class Outer { class First {} interface Second {} }");
    let root = file.scope(File::ROOT_SCOPE_ID).unwrap();
    let outer = find_type_declaration(&file, "Outer");
    let first = find_type_declaration(&file, "First");
    let second = find_type_declaration(&file, "Second");

    assert_eq!(root.child_scopes(), [outer.scope_id]);
    assert_eq!(
        outer.scope.child_scopes(),
        [first.scope_id, second.scope_id]
    );
    assert_eq!(first.scope.parent_scope(), Some(outer.scope_id));
    assert_eq!(second.scope.parent_scope(), Some(outer.scope_id));
}

#[test]
fn interface_and_annotation_members_use_their_containing_scopes() {
    let file = lower_into(
        "interface InterfaceOuter { class InterfaceNested {} }
         @interface AnnotationOuter { class AnnotationNested {} }",
    );
    let interface = find_type_declaration(&file, "InterfaceOuter");
    let interface_nested = find_type_declaration(&file, "InterfaceNested");
    let annotation = find_type_declaration(&file, "AnnotationOuter");
    let annotation_nested = find_type_declaration(&file, "AnnotationNested");

    assert_eq!(
        interface_nested.scope.parent_scope(),
        Some(interface.scope_id)
    );
    assert_eq!(
        annotation_nested.scope.parent_scope(),
        Some(annotation.scope_id)
    );
}

#[test]
fn enum_members_after_constants_use_the_enum_scope() {
    let file = lower_into("enum Outer { VALUE; class Nested {} }");
    let outer = find_type_declaration(&file, "Outer");
    let nested = find_type_declaration(&file, "Nested");

    assert_eq!(nested.scope.parent_scope(), Some(outer.scope_id));
    assert_eq!(nested.declaration.kind, Kind::Class);
}
