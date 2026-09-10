use crate::lower_into;
use beans_lang_java_model::File;

#[test]
fn empty_compilation_unit_has_only_the_root_scope() {
    let file = lower_into("");
    let root = file.scope(File::ROOT_SCOPE_ID).unwrap();

    assert!(file.package_name.is_empty());
    assert!(file.imports.is_empty());
    assert!(file.iter_declarations().next().is_none());
    assert_eq!(file.iter_scopes().count(), 1);
    assert_eq!(root.parent_scope(), None);
    assert!(root.iter_child_scopes().next().is_none());
    assert!(
        file.iter_declarations_in_scope(File::ROOT_SCOPE_ID)
            .next()
            .is_none()
    );
}

#[test]
fn simple_package_name_is_preserved() {
    let file = lower_into("package inventory;");

    assert_eq!(file.package_name.as_slice(), ["inventory"]);
}

#[test]
fn qualified_package_name_preserves_its_components() {
    let file = lower_into("package example.inventory;");

    assert_eq!(file.package_name.as_slice(), ["example", "inventory"]);
}
